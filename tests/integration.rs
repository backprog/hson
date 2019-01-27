#[macro_use]
extern crate lazy_static;

use std::fs::File;
use std::io::prelude::*;

extern crate hson;
use hson::{ Hson, Query, Ops, Search, Cast };


lazy_static! {
    static ref SHORT_DATA: String = {
        let mut data = String::new();
        let mut file = File::open("tests/samples/small.hson").unwrap();
        file.read_to_string(&mut data).unwrap();

        data
    };

    static ref LONG_DATA: String = {
        let mut data = String::new();
        let mut file = File::open("tests/samples/long.hson").unwrap();
        file.read_to_string(&mut data).unwrap();

        data
    };
}

#[test]
fn can_parse () {
    let mut hson = Hson::new();
    assert_eq!(hson.parse(&SHORT_DATA).unwrap(), ());
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
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    assert_eq!(hson.indexes.len(), 25);
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
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("attrs").unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn query_retrieve_in_node () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("div p").unwrap();
    let childs_results = hson.query_on(&results[0], "span", true).unwrap();

    assert_eq!(childs_results.len(), 1);
}

#[test]
fn query_retrieve_in_node_only () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("div").unwrap();
    let childs_results = hson.query_on(&results[0], "attrs", false).unwrap();

    assert_eq!(childs_results.len(), 1);
}

#[test]
fn search () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.search("div").unwrap();
    assert_eq!(results.len(), 3);

    let results = hson.search("div > p attrs id | rate | trusted").unwrap();
    assert_eq!(results.len(), 3);

    let results = hson.search("div > p attrs id = '12' | rate = '3' | trusted").unwrap();
    assert_eq!(results.len(), 2);

    let results = hson.search("div > attrs").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn insertion () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("div p attrs").unwrap();
    assert_eq!(results.len(), 1);

    let child = r#"{
                        "class": ["active", "item"],
                        "name": "foo"
                    }"#;

    assert_eq!(hson.insert(&results[0], 0, child).unwrap(), ());
    assert_eq!(hson.indexes.len(), 29);
    assert_eq!(hson.nodes.keys().len(), 29);
}

#[test]
fn deletion () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("p").unwrap();
    assert_eq!(results.len(), 2);

    assert_eq!(hson.remove(&results[0]).unwrap(), ());
    assert_eq!(hson.indexes.len(), 17);
    assert_eq!(hson.nodes.keys().len(), 17);
}

#[test]
fn vertex () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("div attrs class").unwrap();
    let class = hson.get_vertex(&results[0]).unwrap();
    let values = class.value_as_array().unwrap();

    assert_eq!(values.len(), 4);
    assert_eq!(values[0], "active");
    assert_eq!(values[1], "123");
    assert_eq!(values[2], "0.25864");
    assert_eq!(values[3], "test");
}

#[test]
fn vertex_cast () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("div p attrs").unwrap();
    let attributes = hson.query_on(&results[0], "id", false).unwrap();
    let id = hson.get_vertex(&attributes[0]).unwrap();

    assert_eq!(id.value_as_u64(), Some(12));

    let attributes = hson.query_on(&results[0], "rate", false).unwrap();
    let rate = hson.get_vertex(&attributes[0]).unwrap();

    assert_eq!(rate.value_as_f64(), Some(0.4321));

    let attributes = hson.query_on(&results[0], "trusted", false).unwrap();
    let trusted = hson.get_vertex(&attributes[0]).unwrap();

    assert_eq!(trusted.value_as_bool(), Some(true));
}