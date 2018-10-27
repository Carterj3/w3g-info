// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate derive_new;

#[macro_use]
extern crate log;

extern crate rocket;
extern crate rocket_contrib;

extern crate reqwest;

extern crate bbt;

extern crate byteorder;
extern crate libflate;
extern crate serde; 
extern crate rmp_serde;

extern crate kafka;

extern crate bson;
extern crate mongodb;

pub mod parser;  
pub mod pubsub;
pub mod api;
pub mod rating;

/*
    Common error_chain for all of lib to use so the ? operator passes things around real well.

    Add `use ::errors::*;` to the sub-modules to gain access to it.

    Technically you don't want to do this because it hides the reason for an error and you'll want to use a lot of `links` instead of `foreign_links` but this is way easier.
*/
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
     error_chain!{
        foreign_links {
            Io(::std::io::Error);
            Utf8(::std::string::FromUtf8Error);
            Kafka(::kafka::error::Error);
            ToRmp(::rmp_serde::encode::Error);
            FromRmp(::rmp_serde::decode::Error);
            MongoDb(::mongodb::Error);
            FromBson(::bson::DecoderError);
            ToBson(::bson::EncoderError);
            ParseInt(::std::num::ParseIntError);
            /* NoneError doesn't like to be implemented. Just use `.ok_or("Nothing")?` instead of only `?` */
            // Nothing(::std::option::NoneError);
            Reqwest(::reqwest::Error);
            RocketJson(::rocket_contrib::SerdeError);
        }
    }
}