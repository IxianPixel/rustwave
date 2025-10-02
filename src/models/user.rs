use serde::{Deserialize, Serialize};

use super::deserialize_null_default;

#[derive(Deserialize, Debug, Clone)]
pub struct SoundCloudUsers {
    pub collection: Vec<SoundCloudUser>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct SoundCloudUser {
    pub urn: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub username: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub full_name: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub avatar_url: String,
    pub followers_count: Option<u64>,
}


enum BaseEnum {
    Variant1,
    Variant2,
}

enum ExtendedEnum {
    Variant1,
    Variant2,
    Variant3,
}


fn example(b: impl Into<BaseEnum>) {
    let base: BaseEnum = b.into();
    match base {
        BaseEnum::Variant1 => println!("Variant1"),
        BaseEnum::Variant2 => println!("Variant2"),
    }
}

impl From<ExtendedEnum> for BaseEnum {
    fn from(value: ExtendedEnum) -> Self {
        match value {
            ExtendedEnum::Variant1 => BaseEnum::Variant1,
            ExtendedEnum::Variant2 => BaseEnum::Variant2,
            ExtendedEnum::Variant3 => BaseEnum::Variant2,
        }
    }
}

fn function() {
    let e = ExtendedEnum::Variant1;
    example(e);
    
}


