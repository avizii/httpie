use std::collections::HashMap;
use std::str::FromStr;
use clap::{Parser, Subcommand};
use anyhow::{anyhow, Result};
use reqwest::{Client, header, Response, Url};
use colored::Colorize;
use mime::Mime;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

/// http GET request
#[derive(Parser, Debug)]
struct Get {
    /// http get request url
    #[clap(parse(try_from_str = parse_url))]
    url: String,
}

/// check valid url for get request
fn parse_url(s: &str) -> Result<String> {
    let _url: Url = s.parse()?;
    Ok(s.into())
}

/// http POST request
#[derive(Parser, Debug)]
struct Post {
    /// http post request url
    #[clap(parse(try_from_str = parse_url))]
    url: String,

    /// http post request body
    #[clap(parse(try_from_str = parse_kv_pair))]
    body: Vec<KvPair>,
}

#[derive(Debug, PartialEq)]
struct KvPair {
    k: String,
    v: String,
}

impl FromStr for KvPair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let err = || anyhow!("Failed to parse {}", s);

        let mut it = s.split("=");
        Ok(Self {
            k: (it.next().ok_or_else(err)?).to_string(),
            v: (it.next().ok_or_else(err)?).to_string(),
        })
    }
}

fn parse_kv_pair(s: &str) -> Result<KvPair> {
    Ok(s.parse()?)
}

/// see https://github.com/clap-rs/clap/blob/v3.1.1/examples/tutorial_derive/README.md
#[derive(Subcommand, Debug)]
enum SubCommand {
    Get(Get),
    Post(Post),
}

#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Avizii")]
struct Opts {
    #[clap(subcommand)]
    sub_cmd: SubCommand,
}

fn print_status(response: &Response) {
    let status = format!("{:?} {}", response.version(), response.status()).blue();
    println!("{}\n", status);
}

fn print_header(response: &Response) {
    for (name, value) in response.headers() {
        println!("{}: {:?}", name.to_string().green(), value);
    };
    print!("\n");
}

fn print_syntect(s: &str, ext: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension(ext).unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    for line in LinesWithEndings::from(s) {
        let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
        let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
        println!("{}", escaped);
    }
}

fn print_body(m: Option<Mime>, body: &String) {
    match m {
        Some(v) if v == mime::APPLICATION_JSON => print_syntect(body, "json"),
        Some(v) if v == mime::TEXT_HTML => print_syntect(body, "html"),
        _ => println!("{}", body),
    };
}

fn get_content_type(response: &Response) -> Option<Mime> {
    response.headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().parse().unwrap())
}

async fn print_response(response: Response) -> Result<()> {
    print_status(&response);
    print_header(&response);
    let mime = get_content_type(&response);
    let body = response.text().await?;
    print_body(mime, &body);
    Ok(())
}

async fn get(client: Client, args: &Get) -> Result<()> {
    let response = client.get(&args.url).send().await?; // todo for 2: why not really args, but it is &args
    Ok(print_response(response).await?)
}

async fn post(client: Client, args: &Post) -> Result<()> {
    let mut body = HashMap::new();
    for pair in args.body.iter() {
        body.insert(&pair.k, &pair.v);
    };
    let response = client.post(&args.url).json(&body).send().await?;
    Ok(print_response(response).await?)
}

#[tokio::main]
async fn main() -> Result<()> {
    // cli args parse
    let opts: Opts = Opts::parse();

    let mut headers = header::HeaderMap::new();
    headers.insert("X-POWERED-BY", "Rust".parse()?);
    headers.insert(header::USER_AGENT, "Rust Httpie".parse()?);

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    // `ref` keyword: https://doc.rust-lang.org/std/keyword.ref.html#-vs-ref
    let result = match opts.sub_cmd {
        SubCommand::Get(ref args) => get(client, args).await?,
        SubCommand::Post(ref args) => post(client, args).await?,
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_url_works() {
        assert!(parse_url("abc").is_err());
        assert!(parse_url("https://abc.xyz").is_ok());
        assert!(parse_url("https://abc.org/xyz").is_ok());
    }

    #[test]
    fn parse_kv_pair_works() {
        assert!(parse_kv_pair("a").is_err());
        assert_eq!(
            parse_kv_pair("a=1").unwrap(),
            KvPair {
                k: "a".into(),
                v: "1".into(),
            }
        );
        assert_eq!(
            parse_kv_pair("b=").unwrap(),
            KvPair {
                k: "b".into(),
                v: "".into(),
            }
        );
    }

}