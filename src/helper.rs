use epic_manifest_parser_rs::manifest::shared::FSHAHash;
use sha1::{Sha1, Digest};



pub fn check_hash(data:&Vec<u8>, other:&FSHAHash) -> bool {
    let mut hasher = Sha1::new();
    hasher.update(data);

    FSHAHash::new(hasher.finalize().into()) == *other
}