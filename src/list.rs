use std::env;
use strsim::normalized_damerau_levenshtein;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::fs::File;
use std::path::PathBuf;
use nom::character::complete::alphanumeric0;
use nom::bytes::complete::tag;
use std::io::BufReader;
use nom::IResult;
use nom::multi::separated_list0;
use nom::Parser;
use std::io::prelude::*;

pub struct Server{
input : String
}
impl Server {
pub fn get_list( )-> String{
    let mut input = String::new();
    let config_file = PathBuf::from("~/.ssh/config");
    let file = File::open(config_file).unwrap();
    let mut buf_reader = BufReader::new(file);
    buf_reader.read_to_string(&mut self.input).unwrap();
    input
 
}
pub fn parse_list (input: &str) ->IResult<&'static str, Vec<&'static str>> {

    separated_list0(tag("\n\n"), alphanumeric0).parse(input)

}
}
