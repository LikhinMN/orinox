use libp2p::identity::Keypair;
use std::fs;
use std::io;
use std::path::Path;

pub fn get_or_create_identity() -> Result<Keypair, io::Error> {
    get_or_create_identity_at(Path::new("."))
}

pub fn get_or_create_identity_at(base_path: &Path) -> Result<Keypair, io::Error> {
    let identity_dir = base_path.join(".orinox");
    let identity_path = identity_dir.join("identity.key");

    fs::create_dir_all(&identity_dir)?;

    match fs::read(&identity_path) {
        Ok(content) => Keypair::from_protobuf_encoding(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let key = Keypair::generate_ed25519();
            let encoded = key
                .to_protobuf_encoding()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            fs::write(&identity_path, encoded)?;
            Ok(key)
        }
        Err(e) => Err(e),
    }
}

