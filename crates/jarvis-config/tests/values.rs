//! Doğrulanmış değer tipleri: adlar, yetenekler, `base_url`. Oracle: elle yazılmış tablolar.

use jarvis_config::{BaseUrl, BaseUrlError, Capability, EnvVarName, ProviderName};

#[test]
fn provider_names_follow_the_documented_pattern() {
    for ok in ["a", "nvidia", "local-vision", "a_9", &"a".repeat(32)] {
        let parsed = ProviderName::parse(ok).unwrap();
        assert_eq!(parsed.as_str(), ok);
        assert_eq!(parsed.to_string(), ok);
    }
    for bad in ["", "9a", "A", "-a", "a.b", "nVidia", "ş", &"a".repeat(33)] {
        assert_eq!(ProviderName::parse(bad), None, "{bad:?}");
    }
}

#[test]
fn env_var_names_follow_the_documented_pattern() {
    for ok in ["NVIDIA_API_KEY", "_X", "a1", "K", &"A".repeat(128)] {
        let parsed = EnvVarName::parse(ok).unwrap();
        assert_eq!(parsed.as_str(), ok);
        assert_eq!(parsed.to_string(), ok);
    }
    for bad in ["", "1A", "A-B", "A B", "Ş", &"A".repeat(129)] {
        assert_eq!(EnvVarName::parse(bad), None, "{bad:?}");
    }
}

#[test]
fn capabilities_round_trip_their_file_names() {
    let names = [
        (Capability::Tools, "tools"),
        (Capability::Vision, "vision"),
        (Capability::Streaming, "streaming"),
    ];
    for (capability, name) in names {
        assert_eq!(capability.as_str(), name);
        assert_eq!(capability.to_string(), name);
        assert_eq!(Capability::parse(name), Some(capability));
    }
    assert_eq!(Capability::ALL.to_vec(), names.map(|(c, _)| c).to_vec());
    assert_eq!(Capability::parse("Tools"), None);
}

#[test]
fn base_url_accepts_https_and_loopback_http() {
    for ok in [
        "https://integrate.api.nvidia.com/v1",
        "https://api.openai.com",
        "https://host:8443/v1/",
        "https://[2001:db8::1]:443/v1",
        "http://127.0.0.1:8000/v1",
        "http://127.1.2.3/",
        "http://localhost:11434/v1",
        "http://LOCALHOST/v1",
        "http://[::1]:8080",
        "http://[::1]",
    ] {
        let url = BaseUrl::parse(ok).unwrap_or_else(|e| panic!("{ok}: {e}"));
        assert_eq!(url.as_str(), ok);
        assert_eq!(url.to_string(), ok);
    }
}

#[test]
fn base_url_rejections_name_their_reason() {
    let cases = [
        ("ftp://host/", BaseUrlError::Scheme),
        ("HTTPS://host/", BaseUrlError::Scheme),
        ("host/v1", BaseUrlError::Scheme),
        ("http://api.example.com/v1", BaseUrlError::PlainHttp),
        ("http://127.0.0.1.evil.com/", BaseUrlError::PlainHttp),
        ("http://localhost.evil.com", BaseUrlError::PlainHttp),
        ("http://[::2]/", BaseUrlError::PlainHttp),
        ("http://0.0.0.0:80/", BaseUrlError::PlainHttp),
        ("https://", BaseUrlError::EmptyHost),
        ("https:///v1", BaseUrlError::EmptyHost),
        ("https://:443/", BaseUrlError::EmptyHost),
        ("https://[]/", BaseUrlError::EmptyHost),
        ("https://user:pass@host/", BaseUrlError::Credentials),
        ("http://user@127.0.0.1/", BaseUrlError::Credentials),
        ("https://host:abc/", BaseUrlError::Port),
        ("https://host:/", BaseUrlError::Port),
        ("https://host:70000/", BaseUrlError::Port),
        ("https://host:0/", BaseUrlError::Port),
        ("https://[::1]x/", BaseUrlError::Port),
        ("https://[::1/", BaseUrlError::Port),
        ("https://host/v1?x=1", BaseUrlError::QueryOrFragment),
        ("https://host/#a", BaseUrlError::QueryOrFragment),
        ("https://ho st/", BaseUrlError::Whitespace),
        ("https://host/\n", BaseUrlError::Whitespace),
        ("https://host/\u{7f}", BaseUrlError::Whitespace),
    ];
    for (bad, reason) in cases {
        assert_eq!(BaseUrl::parse(bad), Err(reason), "{bad:?}");
    }
}
