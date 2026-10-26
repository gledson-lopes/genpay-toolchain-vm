use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "genpay.pest"]
pub struct GenpayParser;

pub fn parse_source(
    source_code: &str,
) -> Result<pest::iterators::Pairs<'_, Rule>, pest::error::Error<Rule>> {
    // Ensure Rule::program matches the root rule in your genpay.pest file
    GenpayParser::parse(Rule::program, source_code)
}

