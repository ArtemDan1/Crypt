#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_identities_and_sign() {
        assert_eq!(decode_request(&[11]).unwrap(), Request::Identities);
        assert_eq!(
            decode_request(&[13, 0, 0, 0, 1, 42, 0, 0, 0, 2, 1, 2, 0, 0, 0, 2]).unwrap(),
            Request::Sign {
                key_blob: vec![42],
                data: vec![1, 2],
                flags: 2
            }
        );
    }
    #[test]
    fn unknown_and_extension_messages_are_other() {
        assert_eq!(decode_request(&[27, 1, 2]).unwrap(), Request::Other(27));
        assert_eq!(decode_request(&[17]).unwrap(), Request::Other(17));
    }
    #[test]
    fn truncated_frames_are_errors() {
        for bad in [
            &[][..],
            &[13, 0, 0, 0, 3, 42],
            &[11, 0],
            &[13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ] {
            assert!(decode_request(bad).is_err());
        }
        assert!(read_frame(&mut &b"\0\0"[..]).is_err());
        assert!(read_frame(&mut &b"\0\0\0\x02\x0b"[..]).is_err());
        assert!(read_frame(&mut &b""[..]).unwrap().is_none());
    }
    #[test]
    fn encodes_responses() {
        assert_eq!(encode_response(&Response::Failure), [0, 0, 0, 1, 5]);
        assert_eq!(
            encode_response(&Response::Identities(vec![(vec![42], "t".into())])),
            [0, 0, 0, 15, 12, 0, 0, 0, 1, 0, 0, 0, 1, 42, 0, 0, 0, 1, 116]
        );
        assert_eq!(
            encode_response(&Response::Signature(vec![42])),
            [0, 0, 0, 6, 14, 0, 0, 0, 1, 42]
        );
    }
    #[test]
    fn oversized_frame_rejected() {
        assert!(read_frame(&mut &b"\0\x10\0\0"[..]).is_err());
        assert!(read_frame(&mut &b"\0\0\0\0"[..]).is_err());
    }
}

use std::io::{self, Read};
const MAX_FRAME: usize = 256 * 1024;
#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    Identities,
    Sign {
        key_blob: Vec<u8>,
        data: Vec<u8>,
        flags: u32,
    },
    Other(u8),
}
pub enum Response {
    Identities(Vec<(Vec<u8>, String)>),
    Signature(Vec<u8>),
    Failure,
}
#[derive(Debug, PartialEq, Eq)]
pub struct ProtocolError;
fn take<'a>(input: &mut &'a [u8], n: usize) -> Result<&'a [u8], ProtocolError> {
    if input.len() < n {
        return Err(ProtocolError);
    }
    let (head, rest) = input.split_at(n);
    *input = rest;
    Ok(head)
}
fn u32_value(input: &mut &[u8]) -> Result<u32, ProtocolError> {
    Ok(u32::from_be_bytes(
        take(input, 4)?.try_into().map_err(|_| ProtocolError)?,
    ))
}
fn string(input: &mut &[u8]) -> Result<Vec<u8>, ProtocolError> {
    let len = u32_value(input)? as usize;
    Ok(take(input, len)?.to_vec())
}
pub fn decode_request(mut body: &[u8]) -> Result<Request, ProtocolError> {
    let kind = *take(&mut body, 1)?.first().ok_or(ProtocolError)?;
    let request = match kind {
        11 => Request::Identities,
        13 => Request::Sign {
            key_blob: string(&mut body)?,
            data: string(&mut body)?,
            flags: u32_value(&mut body)?,
        },
        other => return Ok(Request::Other(other)),
    };
    if !body.is_empty() {
        return Err(ProtocolError);
    }
    Ok(request)
}
fn put_string(out: &mut Vec<u8>, s: &[u8]) {
    out.extend_from_slice(&(s.len() as u32).to_be_bytes());
    out.extend_from_slice(s);
}
pub fn encode_response(r: &Response) -> Vec<u8> {
    let mut body = Vec::new();
    match r {
        Response::Failure => body.push(5),
        Response::Signature(sig) => {
            body.push(14);
            put_string(&mut body, sig);
        }
        Response::Identities(keys) => {
            body.push(12);
            body.extend_from_slice(&(keys.len() as u32).to_be_bytes());
            for (blob, comment) in keys {
                put_string(&mut body, blob);
                put_string(&mut body, comment.as_bytes());
            }
        }
    }
    let mut frame = Vec::with_capacity(4 + body.len());
    put_string(&mut frame, &body);
    frame
}
pub fn read_frame(r: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut len = [0u8; 4];
    loop {
        match r.read(&mut len[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    r.read_exact(&mut len[1..])?;
    let len = u32::from_be_bytes(len) as usize;
    if len == 0 || len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid agent frame length",
        ));
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body)?;
    Ok(Some(body))
}
