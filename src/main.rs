fn main() -> Result<(), xmltree::ParseError> {
    use xmltree::Element;
    // use std::fs::File;
    
    let data: &'static str = r##"
    <?xml version="1.0" encoding="UTF-8"?>
    <TODO/>"##;
    
    let names_element = Element::parse(data.as_bytes())?;
    
    println!("{:#?}", names_element);
    return Ok(())
    //{
        // get first `name` element
        //let name = names_element.get_mut_child("name").expect("Can't find name element");
        //name.attributes.insert("suffix".to_owned(), "mr".to_owned());
    //}
    // names_element.write(File::create("result.xml")?)
}
