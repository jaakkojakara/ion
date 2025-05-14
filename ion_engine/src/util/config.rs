use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;

use ion_common::DateTime;

/// A trait for converting Rust config structs into a standardized text format and back.
///
/// The `Config` trait enables serialization and deserialization of configuration structs
/// to/from a human-readable key-value text format. This format supports nested structures,
/// arrays, and various data types including primitives, strings,
/// network addresses, file paths, etc. Support for custom types can be added by implementing the `Config` trait.
///
/// # Format
///
/// The generated format uses sections (denoted by `[section.name]`) for nested structures
/// and key-value pairs for individual fields:
///
/// ```text
/// simple_field = 42
/// string_field = "hello world"
/// boolean_field = true
/// float_field = 123.456
/// array_field = [1,2,3]
///
/// [nested_section]
/// inner_field = "value"
///
/// [deeply.nested.section]
/// deep_field = 123
/// ```
///
/// # Supported Types
///
/// Out of the box, the trait supports:
/// - All primitive numeric types (`u8`, `i32`, `f64`, etc.)
/// - Strings (`String`, `&'static str`)
/// - Booleans
/// - Network types (`SocketAddr`, `IpAddr`, etc.)
/// - File paths (`PathBuf`)
/// - Date/time (`DateTime`)
/// - Vectors of supported types
/// - Optional types (`Option<T>`)
pub trait Config {
    /// Encodes this object's data into a key-value table.
    #[rustfmt::skip]
    fn encode_kv_table(&self, path: &str, table: &mut BTreeMap<String, String>);

    /// Decodes an object from a key-value table.
    #[rustfmt::skip]
    fn decode_kv_table(path: &str, table: &BTreeMap<String, String>) -> Result<Self, ConfigParseError> where Self: Sized;
}

/// Error type for configuration parsing failures.
///
/// This enum represents the different ways that configuration parsing can fail
/// when using the `Config` trait to deserialize text data.
#[derive(Debug, Clone)]
pub enum ConfigParseError {
    /// A field's value could not be parsed to the expected type.
    InvalidFieldType(String),

    /// The input text has invalid syntax.
    InvalidSyntax(String),

    /// A required field is missing from the input.
    MissingData(String),
}

pub(crate) fn config_to_string(config: &dyn Config) -> String {
    let mut table = BTreeMap::new();
    config.encode_kv_table("", &mut table);

    let mut kv_vec: Vec<_> = table.iter().collect();
    kv_vec.sort_by(|x, y| x.0.cmp(y.0));
    kv_vec.sort_by(|x, y| x.0.contains('.').cmp(&y.0.contains('.')));

    let mut string_builder = String::new();
    let mut current_path = "";

    for (key, value) in kv_vec {
        let path_end = key.rfind('.').map(|i| i + 1).unwrap_or(0);
        let path = &key[0..path_end.saturating_sub(1)];

        if path != current_path {
            string_builder.push_str(format!("\n[{}]\n", path).as_str());
            current_path = path;
        }

        string_builder.push_str(format!("{} = {}\n", &key[path_end..], value).as_str());
    }

    string_builder
}

pub(crate) fn config_from_string<T: Config>(string: &str) -> Result<T, ConfigParseError> {
    let mut kv_table = BTreeMap::new();
    let mut current_path = "";

    for line in string
        .split('\n')
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
    {
        if line.starts_with('[') {
            current_path = &line[1..(line.len() - 1)];
        } else {
            let line_vec: Vec<_> = line.splitn(2, '=').collect();
            if line_vec.len() != 2 {
                return Err(ConfigParseError::InvalidSyntax(format!("Invalid line: {}", line)));
            }

            let key = line_vec[0].trim();
            let value = line_vec[1].trim();

            let mut path = current_path.to_owned();
            if !path.is_empty() {
                path.push('.');
            }
            path.push_str(key);

            kv_table.insert(path, value.to_owned());
        }
    }

    T::decode_kv_table("", &kv_table)
}

// ---------------------------------------------------------- //
// ----------------- Config implementations ----------------- //
// ---------------------------------------------------------- //

macro_rules! config_basic_impl {
    ($ty:ty) => {
        impl Config for $ty {
            fn encode_kv_table(&self, name: &str, table: &mut BTreeMap<String, String>) {
                table.insert(name.to_owned(), self.to_string());
            }

            fn decode_kv_table(name: &str, table: &BTreeMap<String, String>) -> Result<Self, ConfigParseError> {
                if let Some(val) = table.get(name) {
                    #[allow(irrefutable_let_patterns)]
                    if let Ok(val) = val.parse() {
                        Ok(val)
                    } else {
                        Err(ConfigParseError::InvalidFieldType(name.to_string()))
                    }
                } else {
                    Err(ConfigParseError::MissingData(name.to_string()))
                }
            }
        }
    };
}

macro_rules! config_vec_impl {
    ($ty:ty) => {
        impl Config for Vec<$ty> {
            fn encode_kv_table(&self, name: &str, table: &mut BTreeMap<String, String>) {
                let str_vec = self
                    .iter()
                    .map(|item| {
                        let mut tmp_map = BTreeMap::new();
                        item.encode_kv_table("tmp", &mut tmp_map);
                        tmp_map.remove("tmp").unwrap()
                    })
                    .collect::<Vec<String>>();

                table.insert(name.to_owned(), format!("[{}]", str_vec.join(",")));
            }

            fn decode_kv_table(name: &str, table: &BTreeMap<String, String>) -> Result<Self, ConfigParseError>
            where
                Self: Sized,
            {
                if let Some(val) = table.get(name) {
                    val[1..(val.len() - 1)]
                        .split(',')
                        .map(|item| {
                            let mut tmp_map = BTreeMap::new();
                            tmp_map.insert("tmp".to_owned(), item.to_owned());
                            <$ty>::decode_kv_table("tmp", &tmp_map)
                        })
                        .collect()
                } else {
                    Err(ConfigParseError::MissingData(name.to_string()))
                }
            }
        }
    };
}

config_basic_impl!(u8);
config_basic_impl!(u16);
config_basic_impl!(u32);
config_basic_impl!(u64);
config_basic_impl!(u128);
config_basic_impl!(i8);
config_basic_impl!(i16);
config_basic_impl!(i32);
config_basic_impl!(i64);
config_basic_impl!(i128);
config_basic_impl!(f32);
config_basic_impl!(f64);
config_basic_impl!(bool);
config_basic_impl!(SocketAddr);
config_basic_impl!(IpAddr);
config_basic_impl!(Ipv4Addr);
config_basic_impl!(Ipv6Addr);

config_vec_impl!(u8);
config_vec_impl!(u16);
config_vec_impl!(u32);
config_vec_impl!(u64);
config_vec_impl!(u128);
config_vec_impl!(i8);
config_vec_impl!(i16);
config_vec_impl!(i32);
config_vec_impl!(i64);
config_vec_impl!(i128);
config_vec_impl!(f32);
config_vec_impl!(f64);
config_vec_impl!(bool);
config_vec_impl!(SocketAddr);
config_vec_impl!(IpAddr);
config_vec_impl!(Ipv4Addr);
config_vec_impl!(Ipv6Addr);
config_vec_impl!(String);
config_vec_impl!(&'static str);

impl Config for PathBuf {
    fn encode_kv_table(&self, name: &str, table: &mut BTreeMap<String, String>) {
        table.insert(name.to_owned(), format!("{:?}", self.display()));
    }

    fn decode_kv_table(name: &str, table: &BTreeMap<String, String>) -> Result<Self, ConfigParseError> {
        if let Some(val) = table.get(name) {
            if !val.starts_with('"') {
                return Err(ConfigParseError::InvalidFieldType(name.to_string()));
            }

            if !val.ends_with('"') {
                return Err(ConfigParseError::InvalidFieldType(name.to_string()));
            }

            match val[1..(val.len() - 1)].to_owned().parse() {
                Ok(res) => Ok(res),
                Err(_) => Err(ConfigParseError::InvalidFieldType(name.to_string())),
            }
        } else {
            Err(ConfigParseError::MissingData(format!("Missing field: {}", name)))
        }
    }
}

impl Config for &'static str {
    fn encode_kv_table(&self, name: &str, table: &mut BTreeMap<String, String>) {
        table.insert(name.to_owned(), format!("\"{}\"", self));
    }

    fn decode_kv_table(name: &str, table: &BTreeMap<String, String>) -> Result<Self, ConfigParseError> {
        if let Some(val) = table.get(name) {
            if !val.starts_with('"') {
                return Err(ConfigParseError::InvalidFieldType(name.to_string()));
            }

            if !val.ends_with('"') {
                return Err(ConfigParseError::InvalidFieldType(name.to_string()));
            }

            let string_to_forget = val[1..(val.len() - 1)].to_owned();
            Ok(Box::leak(string_to_forget.into_boxed_str()))
        } else {
            Err(ConfigParseError::MissingData(format!("Missing field: {}", name)))
        }
    }
}

impl Config for String {
    fn encode_kv_table(&self, name: &str, table: &mut BTreeMap<String, String>) {
        table.insert(name.to_owned(), format!("\"{}\"", self));
    }

    fn decode_kv_table(name: &str, table: &BTreeMap<String, String>) -> Result<Self, ConfigParseError> {
        if let Some(val) = table.get(name) {
            if !val.starts_with('"') {
                return Err(ConfigParseError::InvalidFieldType(name.to_string()));
            }

            if !val.ends_with('"') {
                return Err(ConfigParseError::InvalidFieldType(name.to_string()));
            }

            Ok(val[1..(val.len() - 1)].to_owned())
        } else {
            Err(ConfigParseError::MissingData(format!("Missing field: {}", name)))
        }
    }
}

impl<T: Config> Config for Option<T> {
    fn encode_kv_table(&self, name: &str, table: &mut BTreeMap<String, String>) {
        if let Some(value) = self {
            value.encode_kv_table(name, table);
        }
    }
    fn decode_kv_table(name: &str, table: &BTreeMap<String, String>) -> Result<Self, ConfigParseError> {
        if table.iter().any(|(key, _)| key.contains(name)) {
            let decoded_val = T::decode_kv_table(name, table)?;
            Ok(Some(decoded_val))
        } else {
            Ok(None)
        }
    }
}

impl Config for DateTime {
    fn encode_kv_table(&self, name: &str, table: &mut BTreeMap<String, String>) {
        table.insert(name.to_owned(), self.format_iso8601());
    }

    fn decode_kv_table(name: &str, table: &BTreeMap<String, String>) -> Result<Self, ConfigParseError> {
        if let Some(val) = table.get(name) {
            val.parse()
                .map_err(|_| ConfigParseError::InvalidFieldType(name.to_string()))
        } else {
            Err(ConfigParseError::MissingData(format!("Missing field: {}", name)))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::path::PathBuf;

    use derive_engine::Config;

    use crate::util::config::{Config, ConfigParseError, config_from_string, config_to_string};

    #[derive(Debug, Clone, Config, PartialEq)]
    struct Inner {
        value_1: u64,
        value_2: String,
    }

    #[derive(Debug, Clone, Config, PartialEq)]
    struct Other {
        id: u32,
        name: String,
        inner_inner: Inner,
    }

    #[derive(Debug, Clone, Config, PartialEq)]
    struct TestStruct {
        str: String,
        num: u64,
        vector: Vec<u64>,
        vector2: Vec<&'static str>,
        nested: Inner,
        other: Other,
        boolean: bool,
        addr: SocketAddr,
        path: PathBuf,
        en: EnumTest,
        op1: Option<u32>,
        op2: Option<String>,
        float: f64,
        time: DateTime,
    }

    #[derive(Debug, Clone, Config, PartialEq)]
    enum EnumTest {
        Opt1,
        Opt2,
    }

    fn gen_test_struct() -> TestStruct {
        TestStruct {
            str: "TestValue".to_string(),
            num: 345,
            vector: vec![1, 2, 3],
            vector2: vec!["one", "two"],
            boolean: true,
            float: 0.67353,
            time: DateTime::now(),
            en: EnumTest::Opt2,
            op1: None,
            op2: Some("inside\t_option".to_owned()),
            addr: SocketAddr::from(([192, 168, 1, 112], 6767)),
            path: PathBuf::from("/home/test/asd.txt"),
            nested: Inner {
                value_1: 234234,
                value_2: "INNER TEXT".to_string(),
            },
            other: Other {
                id: 923,
                name: "Bob".to_owned(),
                inner_inner: Inner {
                    value_1: 123,
                    value_2: "INNER INNER TEXT".to_string(),
                },
            },
        }
    }

    #[test]
    fn encoding_and_decoding_config_produces_identical_result() {
        let original = gen_test_struct();
        let encoded = config_to_string(&original);
        println!("{}", encoded);
        let decoded: TestStruct = config_from_string(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn invalid_syntax_fails_correctly() {
        let test_file = "val1 = 123\n val2-45";
        let decoded: Result<TestStruct, _> = config_from_string(test_file);

        match decoded {
            Ok(_) => panic!("Should not succeed"),
            Err(err) => match err {
                ConfigParseError::InvalidFieldType(_) => panic!("Wrong error type {:?}", &err),
                ConfigParseError::InvalidSyntax(_) => {}
                ConfigParseError::MissingData(_) => panic!("Wrong error type {:?}", &err),
            },
        }
    }

    #[test]
    fn invalid_field_type_fails_correctly() {
        let test_file = "value_1 = 123\nvalue_2 = 456";
        let decoded: Result<Inner, _> = config_from_string(test_file);

        match decoded {
            Ok(_) => panic!("Should not succeed"),
            Err(err) => match err {
                ConfigParseError::InvalidFieldType(_) => {}
                ConfigParseError::InvalidSyntax(_) => panic!("Wrong error type {:?}", &err),
                ConfigParseError::MissingData(_) => panic!("Wrong error type {:?}", &err),
            },
        }
    }

    #[test]
    fn missing_field_fails_correctly() {
        let test_file = "value_1 = 123\n value_4 = \"test\"";
        let decoded: Result<Inner, _> = config_from_string(test_file);

        match decoded {
            Ok(_) => panic!("Should not succeed"),
            Err(err) => match err {
                ConfigParseError::InvalidFieldType(_) => panic!("Wrong error type {:?}", &err),
                ConfigParseError::InvalidSyntax(_) => panic!("Wrong error type {:?}", &err),
                ConfigParseError::MissingData(_) => {}
            },
        }
    }
}
