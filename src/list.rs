use std::env;
use strsim::normalized_damerau_levenshtein;
use std::fs::File;
use std::path::PathBuf;
use nom::character::complete::{alphanumeric0, newline, multispace0};
use nom::bytes::complete::tag;
use std::io::BufReader;
use nom::IResult;
use nom::multi::separated_list0;
use nom::Parser;
use std::io::prelude::*;
use nom::bytes::complete::take_until;
use nom::sequence::{delimited, pair, preceded};
// Import (via `use`) the `fmt` module to make it available.
use std::fmt;
pub struct Server {}
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct List {
pub score: f64,
pub hostname: String,
pub alias: String,
}

impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", self.alias)
    }
}
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    PathBuf::from(path)
}

impl Server {
pub fn get_list( )-> String{
    let mut input = String::new();
    let config_file = expand_tilde("~/.ssh/config");
    let file = File::open(config_file).unwrap();
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_string(&mut input).unwrap();
    input
 
}
pub fn parse_list (input: &str) ->IResult<&str, Vec<&str>> {
    delimited(multispace0, separated_list0(pair(newline,newline),take_until("\n\n" )), multispace0).parse(input)
}
pub fn hash_list(list: Vec<&str>) -> Vec<List>{
    let mut servers = Vec::new();
       for item in list {
           let mut server = List{ hostname: "".to_owned(), alias: "".to_owned(), score: 0.0};
           for i in item.split("\n").map(|f| f.trim()){
               let mut i = i.split(" "); 
               let _1 = i.next().unwrap_or("");
               let _2 = i.next().unwrap_or("");
               if _1.starts_with("HostName") {
                   server.hostname = _2.to_owned();

               }else if _1.starts_with("Host"){
                   server.alias = _2.to_owned();
               }
               
           }
               servers.push(server);
       }
       servers
 
}


}

#[cfg(test)]
mod test{
use super::*;

    #[test]
    fn print_debug(){
         let server: Server = Server{};
         let config_file: String = Server::get_list();
        println!("{:#?}", Server::parse_list(&config_file));
    }
    #[test]
    fn print_list_debug(){
     let  server: Server = Server{};
     let  config_file: String = Server::get_list();
     let (_, list) = Server::parse_list(&config_file).unwrap();
    println!("{:#?}", Server::hash_list(list));
    }
}
