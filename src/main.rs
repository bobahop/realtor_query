use chrono::Local;
//use easy_http_request::DefaultHttpRequest;
use rand::Rng;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{prelude::*, BufReader};
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Deserialize, Debug, Serialize)]
struct House {
    name: String,
    status: String,
    price: String,
    query: String,
}

const MAIN_URI: &str = "https://www.realtor.com/realestateandhomes-detail/";
const QUERY_SRC: &str = "C:/rust_projects/realtor_query/target/debug/query_src.txt";
const QUERY_RESULTS: &str = "C:/rust_projects/realtor_query/target/debug/query_results.txt";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/85.0.4183.121 Safari/537.36";

fn main() {
    println!("Starting at {}", Local::now().format("%r"));
    let mut file_out = match OpenOptions::new()
        .append(true)
        .create(true)
        .open(QUERY_RESULTS)
    {
        Ok(val) => val,
        Err(e) => {
            println!("{:?}", e.to_string());
            return;
        }
    };

    let path_in = Path::new(QUERY_SRC);
    let file_in = BufReader::new(File::open(path_in).unwrap());

    let mut search_count = 0;
    for line in file_in.lines() {
        let line = line.unwrap();
        let house: House = serde_json::from_str(&line).unwrap();
        let body: String = get_house(&house.name);
        let status = get_status(&body).to_string();
        //UNKNOWN could now be just sold
        // if status == "UNKNOWN" {
        //     println!("Bot-blocked at {}", Local::now().format("%r"));
        //     return;
        // }
        let mut price = "".to_string();
        let reggie = Regex::new(r#"price">\$[0-9]{3},[0-9]{3}"#).unwrap();
        match status.as_str() {
            "active" | "pending" | "contingent" => {
                if reggie.is_match(&body) {
                    price = match reggie.find(&body) {
                        Some(val) => val.as_str()[7..].to_string(),
                        None => "".to_string(),
                    };
                }
            }
            _ => {}
        }
        println!("{} is {} for {}", house.name, status, price);
        let house_new = House {
            name: house.name,
            status: status,
            price: price,
            query: house.query,
        };
        let line_new = serde_json::to_string(&house_new).unwrap();
        file_out.write_all(line_new.as_bytes()).unwrap();
        file_out.write_all("\n".as_bytes()).unwrap();
        let mut rng = rand::thread_rng();
        let wait_seconds = rng.gen_range(125, 185);
        search_count += 1;
        //was getting bot-blocked after 10 queries, so wait longer after 9 queries
        match search_count % 9 == 0 {
            false => {
                thread::sleep(Duration::from_secs(wait_seconds));
            }
            true => {
                println!(
                    "Waiting for 15 minutes to evade bot-block from {}",
                    Local::now().format("%r")
                );
                thread::sleep(Duration::from_secs(905));
            }
        }
    }
    println!("{}", "Done!");
}

fn get_house(name: &str) -> String {
    let req = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .cookie_store(true)
        .build()
        .unwrap();
    let mut resp = req.get(&(MAIN_URI.to_string() + name)).send().unwrap();
    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf).unwrap();
    buf.iter().map(|c| *c as char).collect::<String>()
    // let req = DefaultHttpRequest::get_from_url_str(MAIN_URI.to_string() + name)
    //     .unwrap()
    //     .send()
    //     .unwrap();
    // req.body.iter().map(|c| *c as char).collect::<String>()
}

fn get_status_tag(status: &str) -> &str {
    match status {
        //old schema
        //"active" => "\"listingStatus\":\"active\"",
        //"pending" => "<span id=\"label-pending\">",
        //"contingent" => "<span id=\"label-contingent\">",
        //"just sold" => "<span id=\"label-sold\"",
        "active" => {
            "<span class=\"jsx-3484526439 label label-dark-transparent\">For Sale - Active</span>"
        }
        "pending" => "<span class=\"jsx-3484526439 label label-red\">Pending</span>",
        "off market" => "<span data-label=\"property-meta-status\">Off Market</span>",
        _ => "UNKNOWN",
    }
}

fn get_status(body: &str) -> &str {
    //need to check pending and contingent first, as they are still "active".
    //check for contingent first, as I think all contingents are also pending.
    // if body.contains(get_status_tag("contingent")) {
    //     "contingent"
    // } else
    //Schema change. Just look for pending and active.
    if body.contains(get_status_tag("pending")) {
        "pending"
    } else if body.contains(get_status_tag("active")) {
        "active"
    } else if body.contains(get_status_tag("off market")) {
        "off market"
    // } else if body.contains(get_status_tag("just sold")) {
    //     "just sold"
    } else {
        //most likely have gotten the bot-block page, although could be a page schema change
        //or could be just sold, don't know it's tag now
        "UNKNOWN"
    }
}
