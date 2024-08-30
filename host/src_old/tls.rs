use core::fmt;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose, SerialNumber,
};
use rustls::pki_types::PrivateKeyDer;
use rustls::server::ResolvesServerCert;
use std::collections::VecDeque;
use std::fmt::Formatter;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::Accept;

use crate::error::Error;

#[derive(Clone)]
pub struct TlsAcceptor {
    acceptor: tokio_rustls::TlsAcceptor,
}

impl TlsAcceptor {
    pub fn new_from_file(cert: &str, key: &str, domains: Vec<String>) -> Result<Self, Error> {
        // let certs = Self::load_certs(cert)?;
        // let key = Self::load_keys(key)?;
        let resolver = Arc::new(FakeCertResolver::new(cert, key, domains)?);
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(resolver);

        Ok(Self {
            acceptor: tokio_rustls::TlsAcceptor::from(Arc::new(config)),
        })
    }
    // fn load_certs(path: &str) -> io::Result<Vec<CertificateDer<'static>>> {
    //     certs(&mut BufReader::new(std::fs::File::open(path)?)).collect()
    // }

    // fn load_keys(path: &str) -> io::Result<PrivateKeyDer<'static>> {
    //     ec_private_keys(&mut BufReader::new(std::fs::File::open(path)?))
    //         .next()
    //         .unwrap()
    //         .map(Into::into)
    // }
    pub fn accept<IO>(&self, stream: IO) -> Accept<IO>
    where
        IO: AsyncRead + AsyncWrite + Unpin,
    {
        self.acceptor.accept(stream)
    }
    pub fn peek_sni_and_alpns(buf: &[u8]) -> Option<(String, Vec<Vec<u8>>)> {
        let extentions = tls_parser::parse_tls_plaintext(&buf)
            .ok()
            .and_then(|msg| VecDeque::from(msg.1.msg).pop_front())
            .and_then(|msg| {
                if let tls_parser::TlsMessage::Handshake(
                    tls_parser::TlsMessageHandshake::ClientHello(h),
                ) = msg
                {
                    tls_parser::parse_tls_client_hello_extensions(h.ext?)
                        .ok()
                        .map(|x| x.1)
                } else {
                    None
                }
            })?;
        let mut sni = None;
        let mut alpns = Vec::new();
        for ext in extentions {
            if let tls_parser::TlsExtension::SNI(s) = ext {
                let (_type, sni_value) = s.first()?;
                sni = String::from_utf8(sni_value.to_vec()).ok();
            } else if let tls_parser::TlsExtension::ALPN(a) = ext {
                alpns.extend(a.iter().map(|x| x.to_vec()));
            }
        }
        sni.map(|s| (s, alpns))
    }
}

pub struct FakeCertResolver {
    domains: Vec<String>,
    ca_cert: Certificate,
    key: KeyPair,
}

impl FakeCertResolver {
    pub fn new(cert_path: &str, key_path: &str, domains: Vec<String>) -> Result<Self, Error> {
        let cert = std::fs::read_to_string(cert_path)?;
        let key = KeyPair::from_pem(&std::fs::read_to_string(key_path)?).unwrap();

        let mut ca_cert_params = CertificateParams::from_ca_cert_pem(&cert).unwrap();
        ca_cert_params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::CrlSign,
            KeyUsagePurpose::KeyCertSign,
        ];
        ca_cert_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        ca_cert_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca_cert = ca_cert_params.self_signed(&key).unwrap();
        std::fs::write("ca.pem", ca_cert.pem())?;
        Ok(Self {
            domains,
            ca_cert,
            key,
        })
    }
}

impl core::fmt::Debug for FakeCertResolver {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("FakeCertResolver").finish()
    }
}

impl ResolvesServerCert for FakeCertResolver {
    fn resolve(
        &self,
        client_hello: rustls::server::ClientHello<'_>,
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        let subject_alt_names = client_hello
            .server_name()
            .map(|s| vec![s.to_string()])
            .unwrap_or(self.domains.clone());

        let mut params = CertificateParams::new(subject_alt_names).ok()?;
        params.not_after = OffsetDateTime::now_utc() + time::Duration::days(365);
        params.not_before = OffsetDateTime::now_utc() - time::Duration::minutes(10);
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        params.serial_number = Some(SerialNumber::from(18277));

        let certificate_key_pair = KeyPair::generate().ok()?;
        // println!("{}", self.ca_cert.pem());
        let ca_cert = self.ca_cert.der().to_owned();
        let cert = params
            .signed_by(&certificate_key_pair, &self.ca_cert, &self.key)
            .ok()?;

        let signing_key = rustls::crypto::aws_lc_rs::sign::any_supported_type(
            &PrivateKeyDer::try_from(certificate_key_pair.serialize_der()).ok()?,
        )
        .ok()?;

        Some(Arc::new(rustls::sign::CertifiedKey::new(
            vec![cert.into(), ca_cert],
            signing_key,
        )))
    }
}
