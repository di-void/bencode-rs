use std::collections::HashMap;

// https://en.wikipedia.org/wiki/Bencode

const INT_DELIM_BEGIN: u8 = b'i';
const DICT_DELIM_BEGIN: u8 = b'd';
const LIST_DELIM_BEGIN: u8 = b'l';
const DELIM_END: u8 = b'e';
const COLON_DELIM: u8 = b':';

#[derive(Debug, PartialEq)]
pub enum BValue {
    Str(String),
    Int(i16),
    List(Vec<BValue>),
    Dict(HashMap<String, BValue>),
    None,
}

pub fn decode(input: &[u8]) -> Result<(BValue, usize), String> {
    if input.len() == 0 {
        return Err(String::from("Decoding Err. Invalid input length."));
    }
    
    match input[0] {
        DELIM_END => {
            // Empty
            Ok((BValue::None, 1))
        }
        // Integers
        INT_DELIM_BEGIN => {
            // move forward to the first digit
            let mut idx = 1;
            let mut n = String::new();

            unsafe {
                let vec = n.as_mut_vec();

                while input[idx] != DELIM_END {
                    vec.push(input[idx]);
                    idx += 1;
                }

                if vec.is_empty() {
                    return Err(String::from("Decoding Error: Empty Integer Not-allowed."));
                }
            }

            let n = n
                .parse::<i16>()
                .map_err(|_e| String::from("Decoding Error: Ill-formatted Integer."))?;

            return Ok((BValue::Int(n), idx + 1));
        }
        LIST_DELIM_BEGIN => {
            // Lists
            let mut idx = 1;
            let mut list = Vec::new();
            loop {
                let (value, consumed) = decode(&input[idx..])?;
                idx += consumed;
                match value {
                    BValue::None => {
                        return Ok((BValue::List(list), idx));
                    }
                    v => {
                        list.push(v);
                    }
                }
            }
        }
        DICT_DELIM_BEGIN => {
            // Dictionaries
            let mut idx = 1;
            let mut dict: HashMap<String, BValue> = HashMap::new();
            let mut key_val = (None, None);

            loop {
                let (value, consumed) = decode(&input[idx..])?;

                match value {
                    BValue::None => {
                        idx += consumed;
                        break;
                    }
                    val => {
                        match val {
                            BValue::Str(s) if key_val.0.is_none() => {
                                key_val.0 = Some(s);
                            }
                            v => {
                                key_val.1 = Some(v);
                            }
                        }
                        idx += consumed;
                    }
                }

                if key_val.0.is_some() && key_val.1.is_some() {
                    let key = key_val.0.unwrap();
                    let val = key_val.1.unwrap();

                    dict.insert(key, val);

                    key_val.0 = None;
                    key_val.1 = None;
                }
            }
            
            Ok((BValue::Dict(dict), idx))
        }
        _ => {
            // Strings
            let mut idx = 0;
            while input[idx] != COLON_DELIM {
                idx += 1;
            }
            let len = String::from_utf8_lossy(&input[..idx]);
            let len = len
                .parse::<usize>()
                .map_err(|_e| String::from("Decoding Error. Invalid string length."))?;
            idx += 1;

            let string = &input
                .get(idx..idx + len)
                .ok_or(String::from("Decoding Error. Invalid string length."))?;
            let string = String::from_utf8(string.to_vec()).unwrap();

            return Ok((BValue::Str(string), idx + len));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_integer_decoding() {
        // Basic integers
        assert_eq!(decode(b"i42e").unwrap().0, BValue::Int(42));
        assert_eq!(decode(b"i0e").unwrap().0, BValue::Int(0));
        assert_eq!(decode(b"i-42e").unwrap().0, BValue::Int(-42));

        // Edge cases
        assert_eq!(decode(b"i042e").unwrap().0, BValue::Int(42)); // Leading zeros not allowed. ALERT! will be normalized
        assert_eq!(decode(b"i-0e").unwrap().0, BValue::Int(0)); // Negative zero not allowed. ALERT! will be normalized
        assert!(decode(b"ie").is_err()); // Empty integer not allowed
        assert!(decode(b"i32be").is_err()); // Non-digit characters not allowed
    }

    #[test]
    fn test_string_decoding() {
        // Basic strings
        assert_eq!(decode(b"4:spam").unwrap().0, BValue::Str("spam".to_string()));
        assert_eq!(decode(b"0:").unwrap().0, BValue::Str("".to_string()));
        assert_eq!(
            decode(b"5:hello").unwrap().0,
            BValue::Str("hello".to_string())
        );

        // Edge cases
        assert!(decode(b"4:spa").is_err()); // String too short
        assert!(decode(b"-1:spam").is_err()); // Negative length
        assert!(decode(b"1x:a").is_err()); // Invalid length delimiter
    }

    #[test]
    fn test_list_decoding() {
        // Empty list
        assert_eq!(decode(b"le").unwrap().0, BValue::List(vec![]));

        // Simple list
        assert_eq!(
            decode(b"l4:spami42ee").unwrap().0,
            BValue::List(vec![BValue::Str("spam".to_string()), BValue::Int(42)])
        );

        // Nested list
        assert_eq!(
            decode(b"ll4:spameli42eee").unwrap().0,
            BValue::List(vec![
                BValue::List(vec![BValue::Str("spam".to_string())]),
                BValue::List(vec![BValue::Int(42)]),
            ])
        );
    }

    #[test]
    fn test_dict_decoding() {
        // Empty dict
        assert_eq!(decode(b"de").unwrap().0, BValue::Dict(HashMap::new()));

        // Simple dict
        let mut expected = HashMap::new();
        expected.insert("spam".to_string(), BValue::Int(42));
        assert_eq!(decode(b"d4:spami42ee").unwrap().0, BValue::Dict(expected));

        // Complex dict
        let mut expected = HashMap::new();
        expected.insert("bar".to_string(), BValue::Str("spam".to_string()));
        expected.insert("foo".to_string(), BValue::Int(42));
        assert_eq!(
            decode(b"d3:bar4:spam3:fooi42ee").unwrap().0,
            BValue::Dict(expected)
        );

        // Edge cases
        assert!(decode(b"d3:foo").is_err()); // Incomplete dict
    }

    #[test]
    fn test_complex_nested_structures() {
        // A complex structure with nested lists and dicts
        let input = b"d8:announce3:url4:infod5:filesld6:lengthi42e4:path4:spamee6:pieces20:aaaaaaaaaaaaaaaaaaaa6:locale2:enee";

        let mut files = HashMap::new();
        files.insert("length".to_string(), BValue::Int(42));
        files.insert("path".to_string(), BValue::Str("spam".to_string()));

        let mut info = HashMap::new();
        info.insert("files".to_string(), BValue::List(vec![BValue::Dict(files)]));
        info.insert(
            "pieces".to_string(),
            BValue::Str("aaaaaaaaaaaaaaaaaaaa".to_string()),
        );
        info.insert("locale".to_string(), BValue::Str("en".to_string()));

        let mut expected = HashMap::new();
        expected.insert("announce".to_string(), BValue::Str("url".to_string()));
        expected.insert("info".to_string(), BValue::Dict(info));

        assert_eq!(decode(input).unwrap().0, BValue::Dict(expected));
    }
}
