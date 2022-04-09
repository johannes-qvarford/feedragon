use std::fs::read_to_string;
use std::fs::File;
use std::path::PathBuf;
use md5::{Md5, Digest};
use tempfile::tempdir;
use std::error::Error;
use xmltree::Element;

pub fn write_element_to_string(element: &Element, hash: &str) -> Result<String, Box<dyn Error>> {
    let dir = tempdir()?;
    let mut hasher = Md5::new();
    hasher.update(hash.as_bytes());
    let v: Vec<_> = hasher.finalize().into_iter().collect();
    let filename = hex::encode(&v[..]);
    let path_buf: PathBuf = dir.path().join(filename);
    element.write(File::create(path_buf.as_path())?)?;
    let response_body = read_to_string(path_buf.as_path())?;
    return Ok(response_body);
}