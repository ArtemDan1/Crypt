use crate::model::ids::RecordId;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct VaultPaths {
    root: PathBuf,
}
impl VaultPaths {
    pub fn new(root: &Path) -> Self {
        Self { root: root.into() }
    }
    pub fn header(&self) -> PathBuf {
        self.root.join("vault.json")
    }
    pub fn device(&self) -> PathBuf {
        self.root.join("device.json")
    }
    pub fn keycheck(&self) -> PathBuf {
        self.root.join("keycheck.bin")
    }
    pub fn seed(&self) -> PathBuf {
        self.root.join("seed.bin")
    }
    pub fn meta(&self) -> PathBuf {
        self.root.join("meta.bin")
    }
    pub fn objects_dir(&self) -> PathBuf {
        self.root.join("objects")
    }
    pub fn object(&self, id: &RecordId) -> PathBuf {
        self.objects_dir().join(format!("{id}.bin"))
    }
}
pub enum Aad {
    KeyCheck,
    Seed,
    Meta,
    Object(RecordId),
}
impl Aad {
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            Self::KeyCheck => b"crypt/v1/keycheck".to_vec(),
            Self::Seed => b"crypt/v1/seed".to_vec(),
            Self::Meta => b"crypt/v1/meta".to_vec(),
            Self::Object(id) => format!("crypt/v1/object/{id}").into_bytes(),
        }
    }
}
