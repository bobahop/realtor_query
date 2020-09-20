use chrono::Local;
use easy_http_request::DefaultHttpRequest;
use rand::Rng;
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
}

const MAIN_URI: &str = "https://www.realtor.com/realestateandhomes-detail/";
const QUERY_SRC: &str = "C:/rust_projects/realtor_query/target/debug/query_src.txt";
const QUERY_RESULTS: &str = "C:/rust_projects/realtor_query/target/debug/query_results.txt";

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
        println!("{} is {}", house.name, status);
        if status == "UNKNOWN" {
            println!("Bot-blocked at {}", Local::now().format("%r"));
            return;
        }
        let house_new = House {
            name: house.name,
            status: status,
        };
        let line_new = serde_json::to_string(&house_new).unwrap();
        file_out.write_all(line_new.as_bytes()).unwrap();
        file_out.write_all("\n".as_bytes()).unwrap();
        let mut rng = rand::thread_rng();
        let wait_seconds = rng.gen_range(65, 125);
        search_count += 1;
        //getting bot-blocked after 10 queries, so wait longer after 9 queries
        match search_count % 9 == 0 {
            false => {
                thread::sleep(Duration::from_secs(wait_seconds));
            }
            true => {
                println!(
                    "Waiting for 30 minutes to evade bot-block from {}",
                    Local::now().format("%r")
                );
                thread::sleep(Duration::from_secs(1805));
            }
        }
    }
    println!("{}", "Done!");
}

fn get_house(name: &str) -> String {
    let req = DefaultHttpRequest::get_from_url_str(MAIN_URI.to_string() + name)
        .unwrap()
        .send()
        .unwrap();
    req.body.iter().map(|c| *c as char).collect::<String>()
}

fn get_status_tag(status: &str) -> &str {
    match status {
        "active" => "\"listingStatus\":\"active\"",
        "pending" => "<span id=\"label-pending\">",
        "contingent" => "<span id=\"label-contingent\">",
        "off market" => "\"status_display\":\"Off Market\"",
        "just sold" => "<span id=\"label-sold\"",
        _ => "UNKNOWN",
    }
}

fn get_status(body: &str) -> &str {
    //need to check pending and contingent first, as they are still "active"
    if body.contains(get_status_tag("pending")) {
        "pending"
    } else if body.contains(get_status_tag("contingent")) {
        "contingent"
    } else if body.contains(get_status_tag("active")) {
        "active"
    } else if body.contains(get_status_tag("off market")) {
        "off market"
    } else if body.contains(get_status_tag("just sold")) {
        "just sold"
    } else {
        //most likely have gotten the bot-block page, although could be a page schema change
        "UNKNOWN"
    }
}
