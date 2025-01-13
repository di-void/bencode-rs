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
    // TODO: remove later
    None
}

pub fn decode(input: &[u8]) -> Result<BValue, String> {
    // check for type delimiters
    // create branches for each Bencode type

    match input[0] {
        INT_DELIM_BEGIN => {
            let mut idx = 0;
            let mut n = String::new();
            idx += 1;

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
            
            let n = n.parse::<i16>().map_err(|_e| String::from("Decoding Error: Ill-formatted Integer."))?;

            return Ok(BValue::Int(n));
        }
        LIST_DELIM_BEGIN => {
            // parse list 
            Ok(BValue::None)
        }
        DICT_DELIM_BEGIN => {
            // parse dictionary
            Ok(BValue::None)
        }
        _ => {
            // Beconde strings
            let mut idx = 0;
            while input[idx] != COLON_DELIM {
                idx += 1;
            }
            let len = String::from_utf8_lossy(&input[..idx]);
            let len = len.parse::<usize>().map_err(|_e| String::from("Decoding Error. Invalid string length."))?;
            idx += 1;

            let string = &input.get(idx..idx + len).ok_or(String::from("Decoding Error. Invalid string length."))?;
            let string = String::from_utf8(string.to_vec()).unwrap();

            return Ok(BValue::Str(string));
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
        assert_eq!(decode(b"i42e").unwrap(), BValue::Int(42));
        assert_eq!(decode(b"i0e").unwrap(), BValue::Int(0));
        assert_eq!(decode(b"i-42e").unwrap(), BValue::Int(-42));

        // Edge cases
        assert_eq!(decode(b"i042e").unwrap(), BValue::Int(42)); // Leading zeros not allowed. ALERT! will be normalized
        assert_eq!(decode(b"i-0e").unwrap(), BValue::Int(0)); // Negative zero not allowed. ALERT! will be normalized
        assert!(decode(b"ie").is_err()); // Empty integer not allowed
        assert!(decode(b"i32be").is_err()); // Non-digit characters not allowed
    }

    #[test]
    fn test_string_decoding() {
        // Basic strings
        assert_eq!(decode(b"4:spam").unwrap(), BValue::Str("spam".to_string()));
        assert_eq!(decode(b"0:").unwrap(), BValue::Str("".to_string()));
        assert_eq!(
            decode(b"5:hello").unwrap(),
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
        assert_eq!(decode(b"le").unwrap(), BValue::List(vec![]));

        // Simple list
        assert_eq!(
            decode(b"l4:spami42ee").unwrap(),
            BValue::List(vec![BValue::Str("spam".to_string()), BValue::Int(42),])
        );

        // Nested list
        assert_eq!(
            decode(b"ll4:spameli42eee").unwrap(),
            BValue::List(vec![
                BValue::List(vec![BValue::Str("spam".to_string())]),
                BValue::List(vec![BValue::Int(42)]),
            ])
        );
    }

    #[test]
    fn test_dict_decoding() {
        // Empty dict
        assert_eq!(decode(b"de").unwrap(), BValue::Dict(HashMap::new()));

        // Simple dict
        let mut expected = HashMap::new();
        expected.insert("spam".to_string(), BValue::Int(42));
        assert_eq!(decode(b"d4:spami42ee").unwrap(), BValue::Dict(expected));

        // Complex dict
        let mut expected = HashMap::new();
        expected.insert("bar".to_string(), BValue::Str("spam".to_string()));
        expected.insert("foo".to_string(), BValue::Int(42));
        assert_eq!(
            decode(b"d3:bar4:spam3:fooi42ee").unwrap(),
            BValue::Dict(expected)
        );

        // Edge cases
        assert!(decode(b"d3:fooi42e3:bar4:spame").is_err()); // Unordered keys
        assert!(decode(b"d3:foo").is_err()); // Incomplete dict
    }

    #[test]
    fn test_complex_nested_structures() {
        // A complex structure with nested lists and dicts
        let input = b"d8:announce3:url4:infod5:filesld6:lengthi42e4:path4:spameed6:pieces20:aaaaaaaaaaaaaaaaaaaa6:locale2:enee";

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

        assert_eq!(decode(input).unwrap(), BValue::Dict(expected));
    }
}
