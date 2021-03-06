use std::fs::File;
use std::io::prelude::*;

#[macro_use]
extern crate lazy_static;

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

    static ref NUM_DATA: String = {
        let mut data = String::new();
        let mut file = File::open("tests/samples/num.hson").unwrap();
        file.read_to_string(&mut data).unwrap();

        data
    };

    static ref SIMPLE_DATA: String = {
        let mut data = String::new();
        let mut file = File::open("tests/samples/simple.hson").unwrap();
        file.read_to_string(&mut data).unwrap();

        data
    };

    static ref HTML_DATA: String = {
        let mut data = String::new();
        let mut file = File::open("tests/samples/html-1.hson").unwrap();
        file.read_to_string(&mut data).unwrap();

        data
    };

    static ref NESTED_DATA: String = {
        let mut data = String::new();
        let mut file = File::open("tests/samples/nested.hson").unwrap();
        file.read_to_string(&mut data).unwrap();

        data
    };

    static ref ARRAY_DATA: String = {
        let mut data = String::new();
        let mut file = File::open("tests/samples/array.hson").unwrap();
        file.read_to_string(&mut data).unwrap();

        data
    };

    static ref INTRICATE_DATA: String = {
        let mut data = String::new();
        let mut file = File::open("tests/samples/intricate.hson").unwrap();
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
fn can_parse_uri () {
    let mut hson = Hson::new();
    assert_eq!(hson.parse(&HTML_DATA).unwrap(), ());
}

#[test]
fn can_parse_intricate () {
    let mut hson = Hson::new();
    assert_eq!(hson.parse(&INTRICATE_DATA).unwrap(), ());
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
fn parse_array () {
    let mut hson = Hson::new();
    assert_eq!(hson.parse(&ARRAY_DATA).unwrap(), ());

    assert_eq!(hson.indexes.len(), 17);
}

#[test]
fn has_nodes_number_simple () {
    let mut hson = Hson::new();
    hson.parse(&SIMPLE_DATA).unwrap();

    assert_eq!(hson.indexes.len(), 6);
}

#[test]
fn has_nodes_number_small () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    assert_eq!(hson.indexes.len(), 25);
}

#[test]
fn has_nodes_number_num () {
    let mut hson = Hson::new();
    hson.parse(&NUM_DATA).unwrap();

    assert_eq!(hson.indexes.len(), 47);
}

#[test]
fn has_nodes_number_long () {
    let mut hson = Hson::new();
    hson.parse(&LONG_DATA).unwrap();

    assert_eq!(hson.indexes.len(), 295);
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
    let childs_results = hson.query_on(results[0], "span", true).unwrap();

    assert_eq!(childs_results.len(), 1);
}

#[test]
fn query_retrieve_in_node_only () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("div").unwrap();
    let childs_results = hson.query_on(results[0], "attrs", false).unwrap();

    assert_eq!(childs_results.len(), 1);
}

#[test]
fn search_on_small () {
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
fn search_on_long () {
    let mut hson = Hson::new();
    hson.parse(&LONG_DATA).unwrap();

    let results = hson.search("div").unwrap();
    assert_eq!(results.len(), 7);

    let results = hson.search("li>attrs").unwrap();
    assert_eq!(results.len(), 17);
}

#[test]
fn search_in_nested_array () {
    let mut hson = Hson::new();
    hson.parse(&NESTED_DATA).unwrap();

    assert_eq!(hson.indexes.len(), 7);

    let results = hson.search("p b").unwrap();
    assert_eq!(results.len(), 2);

    let results = hson.search("div>b").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn search_in_node () {
    let mut hson = Hson::new();
    hson.parse(&LONG_DATA).unwrap();

    let results = hson.search("p>attrs").unwrap();
    let ids = hson.search_in(results[1], "id").unwrap();

    assert_eq!(ids.len(), 1);

    let vertex = hson.get_vertex(ids[0]).unwrap();
    assert_eq!(vertex.value_as_string().unwrap(), "test-2");
}

#[test]
fn search_in_node_only () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.search("div").unwrap();
    assert_eq!(results.len(), 3);

    let res = hson.search_in(results[0], ">attrs").unwrap();
    assert_eq!(res.len(), 1);
}

#[test]
fn search_in_array () {
    let mut hson = Hson::new();
    hson.parse(&ARRAY_DATA).unwrap();

    let results = hson.search("id").unwrap();
    assert_eq!(results.len(), 4);
}

#[test]
fn insertion_long () {
    let mut hson = Hson::new();
    hson.parse(&LONG_DATA).unwrap();

    assert_eq!(hson.indexes.len(), 295);

    let results = hson.search("li>attrs>id").unwrap();
    assert_eq!(results.len(), 0);

    let results = hson.search("li>attrs").unwrap();

    let child = r#"{
                        "class": ["active", "item"],
                        "id": "my-id",
                        "rel": {
                            "name": "test",
                            "id": "43526",
                            "pos": 2
                        }
                    }"#;

    assert_eq!(hson.insert(results[0], 1, child).unwrap(), ());
    assert_eq!(hson.indexes.len(), 303);
    assert_eq!(hson.nodes.keys().len(), 303);

    let results = hson.search("li>attrs>id").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn insertion_small () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    assert_eq!(hson.indexes.len(), 25);

    let results = hson.query("div p attrs").unwrap();
    assert_eq!(results.len(), 1);

    let child = r#"{
                        "class": ["active", "item"],
                        "name": "foo"
                    }"#;

    assert_eq!(hson.insert(results[0], 0, child).unwrap(), ());
    assert_eq!(hson.indexes.len(), 29);
    assert_eq!(hson.nodes.keys().len(), 29);
}

#[test]
fn deletion () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    assert_eq!(hson.indexes.len(), 25);

    let results = hson.query("p").unwrap();
    assert_eq!(results.len(), 2);

    assert_eq!(hson.remove(results[0]).unwrap(), ());
    assert_eq!(hson.indexes.len(), 17);
    assert_eq!(hson.nodes.keys().len(), 17);
}

#[test]
fn replacement () {
    let mut hson = Hson::new();
    hson.parse(&HTML_DATA).unwrap();

    let results = hson.search("attrs class").unwrap();
    assert_eq!(results.len(), 3);

    let replace = r#"{
        "data": "test"
    }"#;
    hson.replace(results[0], replace).unwrap();

    let results = hson.search("attrs data").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn vertex () {
    let mut hson = Hson::new();
    hson.parse(&SHORT_DATA).unwrap();

    let results = hson.query("div attrs class").unwrap();
    let class = hson.get_vertex(results[0]).unwrap();
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
    let attributes = hson.query_on(results[0], "id", false).unwrap();
    let id = hson.get_vertex(attributes[0]).unwrap();

    assert_eq!(id.value_as_u64(), Some(12));

    let attributes = hson.query_on(results[0], "rate", false).unwrap();
    let rate = hson.get_vertex(attributes[0]).unwrap();

    assert_eq!(rate.value_as_f64(), Some(0.4321));

    let attributes = hson.query_on(results[0], "trusted", false).unwrap();
    let trusted = hson.get_vertex(attributes[0]).unwrap();

    assert_eq!(trusted.value_as_bool(), Some(true));
}