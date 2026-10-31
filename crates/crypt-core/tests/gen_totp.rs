use crypt_core::{
    r#gen::{PasswordOptions, generate_password},
    totp::{TotpAlg, TotpParams, parse_totp, totp_at},
};
#[test]
fn password_respects_length_and_classes() {
    for _ in 0..200 {
        let p = generate_password(&PasswordOptions::default()).unwrap();
        assert_eq!(p.len(), 24);
        assert!(p.bytes().any(|b| b.is_ascii_lowercase()));
        assert!(p.bytes().any(|b| b.is_ascii_uppercase()));
        assert!(p.bytes().any(|b| b.is_ascii_digit()));
        assert!(p.contains(|c| "!\"#$%&'*+,./:;=?@\\^`|~".contains(c)));
        let p = generate_password(&PasswordOptions {
            avoid_ambiguous: true,
            ..Default::default()
        })
        .unwrap();
        assert!(!p.contains(|c| "0O1lI|".contains(c)));
    }
    let p = generate_password(&PasswordOptions {
        length: 8,
        upper: false,
        digits: false,
        symbols: false,
        ..Default::default()
    })
    .unwrap();
    assert!(p.bytes().all(|b| b.is_ascii_lowercase()));
}
#[test]
fn password_rejects_bad_options() {
    for length in [3, 129] {
        assert!(
            generate_password(&PasswordOptions {
                length,
                ..Default::default()
            })
            .is_err()
        );
    }
    assert!(
        generate_password(&PasswordOptions {
            lower: false,
            upper: false,
            digits: false,
            symbols: false,
            ..Default::default()
        })
        .is_err()
    );
}
#[test]
fn totp_rfc6238_vectors() {
    for (algorithm, secret, t, expected) in [
        (TotpAlg::Sha1, "12345678901234567890", 59, "94287082"),
        (
            TotpAlg::Sha1,
            "12345678901234567890",
            1111111109,
            "07081804",
        ),
        (
            TotpAlg::Sha1,
            "12345678901234567890",
            20000000000,
            "65353130",
        ),
        (
            TotpAlg::Sha256,
            "12345678901234567890123456789012",
            59,
            "46119246",
        ),
        (
            TotpAlg::Sha512,
            "1234567890123456789012345678901234567890123456789012345678901234",
            59,
            "90693936",
        ),
    ] {
        let code = totp_at(
            &TotpParams {
                secret: secret.as_bytes().to_vec().into(),
                algorithm,
                digits: 8,
                period: 30,
            },
            t,
        );
        assert_eq!(code.code, expected);
        if t == 59 {
            assert_eq!(code.remaining, 1);
        }
    }
}
#[test]
fn totp_parses_base32_and_otpauth() {
    assert_eq!(
        *parse_totp("gezd gnbv gy3t qojq").unwrap().secret,
        *parse_totp("GEZDGNBVGY3TQOJQ").unwrap().secret
    );
    let p = parse_totp(
        "otpauth://totp/X:me?secret=GEZDGNBVGY3TQOJQ&digits=8&period=60&algorithm=SHA256",
    )
    .unwrap();
    assert_eq!((p.digits, p.period, p.algorithm), (8, 60, TotpAlg::Sha256));
    for bad in [
        "",
        "!!!",
        "otpauth://hotp/x?secret=GEZD",
        "otpauth://totp/x?secret=GEZD&digits=9",
        "otpauth://totp/x?secret=GEZD&period=0",
    ] {
        assert!(parse_totp(bad).is_err());
    }
}
#[test]
fn totp_rejects_malformed_padding() {
    assert!(parse_totp("GEZDGNBVGY3TQOJQ=").is_err());
    assert!(parse_totp("MY=======").is_err());
    assert!(parse_totp("M=Y=====").is_err());
    assert_eq!(&*parse_totp("MY======").unwrap().secret, b"f");
    assert_eq!(&*parse_totp("MY").unwrap().secret, b"f");
}
