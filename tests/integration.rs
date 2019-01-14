use std::fs::File;
use std::io::prelude::*;

extern crate hson;
use hson::{ Hson, Query, Ops };

#[test]
fn can_parse () {
    let mut data = String::new();
    let mut file = File::open("tests/samples/small.hson").unwrap();
    file.read_to_string(&mut data).unwrap();

    let mut hson = Hson::new();
    assert_eq!(hson.parse(&data).unwrap(), ());
}

#[test]
#[should_panic]
fn cant_parse () {
    let data = r#"{
            "div": {
                "class": [],
                "text": "World
            },
            "ul": {
                "class": ["active","test"]
            }
        }"#;

    let mut hson = Hson::new();
    hson.parse(&data).unwrap();
}

#[test]
fn has_nodes_number () {
    let mut data = String::new();
    let mut file = File::open("tests/samples/small.hson").unwrap();
    file.read_to_string(&mut data).unwrap();

    let mut hson = Hson::new();
    hson.parse(&data).unwrap();

    assert_eq!(hson.indexes.len(), 16);
}

#[test]
#[should_panic]
fn invalid_chars () {
    let data = r#"{
            "div": {
                class: [],
                "text": "World
            },
            "ul": {
                "class": ["active","test"]
            }
        }"#;

    let mut hson = Hson::new();
    hson.parse(&data).unwrap();
}

#[test]
fn query_retrieve_elements () {
    let mut data = String::new();
    let mut file = File::open("tests/samples/small.hson").unwrap();
    file.read_to_string(&mut data).unwrap();

    let mut hson = Hson::new();
    hson.parse(&data).unwrap();

    let results = hson.query("attrs").unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn insertion () {
    let mut data = String::new();
    let mut file = File::open("tests/samples/small.hson").unwrap();
    file.read_to_string(&mut data).unwrap();

    let mut hson = Hson::new();
    hson.parse(&data).unwrap();

    let results = hson.query("div p attrs").unwrap();
    assert_eq!(results.len(), 1);

    let child = r#"{
                        "class": ["active", "item"],
                        "name": "foo"
                    }"#;

    assert_eq!(hson.insert(&results[0], 0, child).unwrap(), ());
    assert_eq!(hson.indexes.len(), 18);
    assert_eq!(hson.nodes.keys().len(), 18);
}

#[test]
fn deletion () {
    let mut data = String::new();
    let mut file = File::open("tests/samples/small.hson").unwrap();
    file.read_to_string(&mut data).unwrap();

    let mut hson = Hson::new();
    hson.parse(&data).unwrap();

    let results = hson.query("p").unwrap();
    assert_eq!(results.len(), 2);

    assert_eq!(hson.remove(&results[0]).unwrap(), ());
    assert_eq!(hson.indexes.len(), 12);
    assert_eq!(hson.nodes.keys().len(), 12);
}