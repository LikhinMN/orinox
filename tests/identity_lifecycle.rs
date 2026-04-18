use orinox::identity::get_or_create_identity_at;
use std::fs;
use std::io;
use tempfile::tempdir;

#[test]
fn creates_identity_file_when_missing() {
    let tmp = tempdir().expect("temp dir should be created");

    let key = get_or_create_identity_at(tmp.path()).expect("identity should be created");

    assert!(tmp.path().join(".orinox/identity.key").exists());
    assert!(
        key.to_protobuf_encoding()
            .expect("created key should be encodable")
            .len()
            > 0
    );
}

#[test]
fn reloads_the_same_identity_key() {
    let tmp = tempdir().expect("temp dir should be created");

    let first = get_or_create_identity_at(tmp.path()).expect("identity should be created");
    let second = get_or_create_identity_at(tmp.path()).expect("identity should be loaded");

    let first_bytes = first
        .to_protobuf_encoding()
        .expect("first key should be encodable");
    let second_bytes = second
        .to_protobuf_encoding()
        .expect("second key should be encodable");

    assert_eq!(first_bytes, second_bytes);
}

#[test]
fn returns_invalid_data_for_corrupted_identity_file() {
    let tmp = tempdir().expect("temp dir should be created");
    let identity_dir = tmp.path().join(".orinox");
    let identity_path = identity_dir.join("identity.key");

    fs::create_dir_all(&identity_dir).expect("identity dir should be created");
    fs::write(&identity_path, b"not-a-protobuf-key").expect("corrupted key should be written");

    let err = get_or_create_identity_at(tmp.path()).expect_err("corrupted key should fail to load");

    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

