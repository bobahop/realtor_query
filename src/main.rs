use chrono::Local;
use rand::Rng;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
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

#[derive(Debug)]
struct BobError {
    text: String,
}
impl fmt::Display for BobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.text)
    }
}

impl Error for BobError {}

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

    let req = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .cookie_store(true)
        .build()
        .unwrap();

    let mut body_num: u32 = 0;
    let lines: Vec<_> = file_in.lines().collect();
    let line_count = lines.len();
    for (line_index, line) in lines.into_iter().enumerate() {
        let line = line.unwrap();
        let house: House = serde_json::from_str(&line).unwrap();
        let body = get_body(&req, &house.name);
        match body {
            Err(_) => {
                return;
            }
            _ => {}
        }
        let body = body.unwrap();
        let status = get_status(&body).to_string();
        //UNKNOWN
        if status == "UNKNOWN" {
            let reason = get_unknown_reason(&body);
            println!("{} at {}", reason, Local::now().format("%r"));
            body_num += 1;
            print_unknown_body(&body, body_num);
            return;
        }
        let price = get_price(&body, &status);
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

        //don't wait if we're done
        let line_num = line_index + 1;
        if line_num == line_count {
            break;
        }

        let mut rng = rand::thread_rng();
        let wait_seconds = rng.gen_range(125, 185);
        //was getting bot-blocked after 10 queries, so wait longer after 9 queries
        match line_num % 9 == 0 {
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

fn get_body(req: &reqwest::blocking::Client, house_name: &str) -> Result<String, BobError> {
    let mut body: String = get_house(&req, house_name);
    match body.as_str() {
        "Error" => {
            return Err(BobError {
                text: "Error".to_string(),
            });
        }
        "Timeout" => {
            //give it one more try after five minutes
            thread::sleep(Duration::from_secs(305));
            body = get_house(&req, house_name);
            match body.as_str() {
                "Error" | "Timeout" => {
                    return Err(BobError {
                        text: "Error".to_string(),
                    })
                }
                _ => {}
            };
        }
        _ => {}
    };
    Ok(body)
}
fn get_house(req: &reqwest::blocking::Client, house_name: &str) -> String {
    let mut resp = match req.get(&(MAIN_URI.to_string() + house_name)).send() {
        Ok(success_val) => success_val,
        Err(fail_error) => {
            let fail_error = fail_error as reqwest::Error;
            if fail_error.is_timeout() {
                println!("Timeout at {}", Local::now().format("%r"));
                return "Timeout".to_string();
            } else {
                println!(
                    "{} at {}",
                    fail_error.to_string(),
                    Local::now().format("%r")
                );
            }
            return "Error".to_string();
        }
    };
    let mut buf: Vec<u8> = vec![];
    resp.copy_to(&mut buf).unwrap();
    buf.iter().map(|c| *c as char).collect::<String>()
}

fn get_status_tag(status: &str) -> &str {
    match status {
        //old schema
        //"active" => "\"listingStatus\":\"active\"",
        //"pending" => "<span id=\"label-pending\">",
        //"contingent" => "<span id=\"label-contingent\">",
        //"just sold" => "<span id=\"label-sold\"",
        "active1" => {
            "<span class=\"jsx-3484526439 label label-dark-transparent\">For Sale - Active</span>"
        }
        "active2" => "<span data-label=\"property-meta-active\">Active</span>",
        "pending1" => "<span class=\"jsx-3484526439 label label-red\">Pending</span>",
        "pending2" => "<span id=\"label-pending\">Pending</span>",
        "just sold" => "<span id=\"label-sold\">",
        "off market1" => "<span data-label=\"property-meta-status\">Off Market</span>",
        "off market2" => {
            "<span id=\"pdp-meta-hero-tag\" data-label=\"property-meta-status\">Off Market</span>"
        }
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
    if body.contains(get_status_tag("pending1")) {
        "pending"
    } else if body.contains(get_status_tag("pending2")) {
        "pending"
    } else if body.contains(get_status_tag("active1")) {
        "active"
    } else if body.contains(get_status_tag("active2")) {
        "active"
    } else if body.contains(get_status_tag("just sold")) {
        "just sold"
    } else if body.contains(get_status_tag("off market1")) {
        "off market"
    } else if body.contains(get_status_tag("off market2")) {
        "off market"
    } else {
        //most likely have gotten the bot-block page, although could be a page schema change
        "UNKNOWN"
    }
}

fn print_unknown_body(body: &str, body_num: u32) {
    let out_file = "C:/rust_projects/realtor_query/target/debug/";
    let mut file_out = match OpenOptions::new()
        .append(true)
        .create(true)
        .open(out_file.to_string() + &body_num.to_string() + ".html")
    {
        Ok(val) => val,
        Err(e) => {
            println!("{:?}", e.to_string());
            return;
        }
    };
    file_out.write_all(body.as_bytes()).unwrap();
}

fn get_price(body: &str, status: &str) -> String {
    let mut _price = "".to_string();
    if status != "active" && status != "pending" && status != "contingent" {
        return _price;
    }
    let reggie = Regex::new(r#"price">\$[0-9]{3},[0-9]{3}"#).unwrap();
    if reggie.is_match(&body) {
        _price = match reggie.find(&body) {
            Some(val) => val.as_str()[7..].to_string(),
            None => "".to_string(),
        };
    }
    if _price == "" {
        //need to match with all its line breaks and spaces
        // <span itemprop="price" content="185000">
        //                         $185,000
        //                       </span>
        let reggie =
            Regex::new(r#"<span itemprop="price" content="[0-9]{6}">\s+\$[0-9]{3},[0-9]{3}"#)
                .unwrap();
        if reggie.is_match(&body) {
            _price = match reggie.find(body) {
                Some(val) => val.as_str()[40..].to_string(),
                None => "".to_string(),
            };
            if !_price.is_empty() {
                let reggie = Regex::new(r#"\$[0-9]{3},[0-9]{3}"#).unwrap();
                _price = match reggie.find(&_price) {
                    Some(val) => val.as_str().to_string(),
                    None => "".to_string(),
                };
            }
        }
    }
    _price
}

fn get_unknown_reason(body: &str) -> String {
    if body.contains("<title>Pardon Our Interruption</title>") {
        "Bot-Blocked".to_string()
    } else if body.contains("<title>Service Unavailable</title>") {
        "Service Unavailable".to_string()
    } else {
        "Possible schema change".to_string()
    }
}
