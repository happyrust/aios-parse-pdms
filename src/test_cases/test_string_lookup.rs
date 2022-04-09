use crate::pdms_types::StringLookupTable;

#[test]
pub fn test_lookup() {
    let mut lookup = StringLookupTable::new();
    lookup.add_str("test");
    lookup.add_str("test");
    lookup.add_str("test1");
    lookup.add_str("test1");
    lookup.add_str("test2");
    //dbg!(lookup.lookup.len());

    lookup.serialize_to_default_json_file();
}