use libc::{size_t, c_void};
use core_foundation::array::CFArray;
use core_foundation::base::{TCFType, Boolean};
use core_foundation_sys::base::{OSStatus};
#[cfg(feature = "OSX_10_8")]
use core_foundation_sys::base::{kCFAllocatorDefault, CFRelease};
use security_framework_sys::base::{errSecSuccess, errSecIO, errSecBadReq};
use security_framework_sys::secure_transport::*;
use std::io;
use std::io::prelude::*;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::slice;
use std::result;

use {cvt, ErrorNew, CipherSuiteInternals};
use base::{Result, Error};
use certificate::SecCertificate;
use cipher_suite::CipherSuite;
use identity::SecIdentity;
use trust::SecTrust;

#[derive(Debug, Copy, Clone)]
pub enum ProtocolSide {
    Server,
    Client,
}

impl ProtocolSide {
    #[cfg(feature = "OSX_10_8")]
    fn to_raw(&self) -> SSLProtocolSide {
        match *self {
            ProtocolSide::Server => SSLProtocolSide::kSSLServerSide,
            ProtocolSide::Client => SSLProtocolSide::kSSLClientSide,
        }
    }
}

#[derive(Debug)]
pub enum HandshakeError<S> {
    Failure(Error),
    ServerAuthCompleted(MidHandshakeSslStream<S>),
}

#[derive(Debug)]
pub struct MidHandshakeSslStream<S>(SslStream<S>);

impl<S> MidHandshakeSslStream<S> {
    pub fn context(&self) -> &SslContext {
        &self.0.ctx
    }

    pub fn handshake(self) -> result::Result<SslStream<S>, HandshakeError<S>> {
        unsafe {
            match SSLHandshake(self.0.ctx.0) {
                errSecSuccess => Ok(self.0),
                errSSLPeerAuthCompleted => {
                    Err(HandshakeError::ServerAuthCompleted(self))
                }
                err => Err(HandshakeError::Failure(Error::new(err))),
            }
        }
    }
}

#[derive(Debug)]
pub enum SessionState {
    Idle,
    Handshake,
    Connected,
    Closed,
    Aborted,
}

impl SessionState {
    fn from_raw(raw: SSLSessionState) -> SessionState {
        match raw {
            SSLSessionState::kSSLIdle => SessionState::Idle,
            SSLSessionState::kSSLHandshake => SessionState::Handshake,
            SSLSessionState::kSSLConnected => SessionState::Connected,
            SSLSessionState::kSSLClosed => SessionState::Closed,
            SSLSessionState::kSSLAborted => SessionState::Aborted,
        }
    }
}

pub struct SslContext(SSLContextRef);

impl fmt::Debug for SslContext {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = fmt.debug_struct("SslContext");
        if let Ok(state) = self.state() {
            builder.field("state", &state);
        }
        builder.finish()
    }
}

unsafe impl Send for SslContext {}

impl SslContext {
    #[cfg(not(feature = "OSX_10_8"))]
    pub fn new(side: ProtocolSide) -> Result<SslContext> {
        unsafe {
            let is_server = match side {
                ProtocolSide::Server => 1,
                ProtocolSide::Client => 0,
            };

            let mut ctx = ptr::null_mut();
            try!(cvt(SSLNewContext(is_server, &mut ctx)));
            Ok(SslContext(ctx))
        }
    }

    #[cfg(feature = "OSX_10_8")]
    pub fn new(side: ProtocolSide) -> Result<SslContext> {
        unsafe {
            let ctx = SSLCreateContext(kCFAllocatorDefault,
                                       side.to_raw(),
                                       SSLConnectionType::kSSLStreamType);
            Ok(SslContext(ctx))
        }
    }

    pub fn set_peer_domain_name(&mut self, peer_name: &str) -> Result<()> {
        unsafe {
            // SSLSetPeerDomainName doesn't need a null terminated string
            cvt(SSLSetPeerDomainName(self.0,
                                     peer_name.as_ptr() as *const _,
                                     peer_name.len() as size_t))
        }
    }

    pub fn set_certificate(&mut self,
                           identity: &SecIdentity,
                           certs: &[SecCertificate])
                           -> Result<()> {
        let mut arr = vec![identity.as_CFType()];
        arr.extend(certs.iter().map(|c| c.as_CFType()));
        let certs = CFArray::from_CFTypes(&arr);

        unsafe {
            cvt(SSLSetCertificate(self.0, certs.as_concrete_TypeRef()))
        }
    }

    pub fn set_break_on_server_auth(&mut self, break_on_server_auth: bool) -> Result<()> {
        unsafe {
            cvt(SSLSetSessionOption(self.0,
                                    SSLSessionOption::kSSLSessionOptionBreakOnServerAuth,
                                    break_on_server_auth as Boolean))
        }
    }

    pub fn supported_ciphers(&self) -> Result<Vec<CipherSuite>> {
        unsafe {
            let mut num_ciphers = 0;
            try!(cvt(SSLGetNumberSupportedCiphers(self.0, &mut num_ciphers)));
            let mut ciphers = Vec::with_capacity(num_ciphers as usize);
            ciphers.set_len(num_ciphers as usize);
            try!(cvt(SSLGetSupportedCiphers(self.0, ciphers.as_mut_ptr(), &mut num_ciphers)));
            Ok(ciphers.iter().map(|c| CipherSuite::from_raw(*c).unwrap()).collect())
        }
    }

    pub fn enabled_ciphers(&self) -> Result<Vec<CipherSuite>> {
        unsafe {
            let mut num_ciphers = 0;
            try!(cvt(SSLGetNumberEnabledCiphers(self.0, &mut num_ciphers)));
            let mut ciphers = Vec::with_capacity(num_ciphers as usize);
            ciphers.set_len(num_ciphers as usize);
            try!(cvt(SSLGetEnabledCiphers(self.0, ciphers.as_mut_ptr(), &mut num_ciphers)));
            Ok(ciphers.iter().map(|c| CipherSuite::from_raw(*c).unwrap()).collect())
        }
    }

    pub fn set_enabled_ciphers(&mut self, ciphers: &[CipherSuite]) -> Result<()> {
        let ciphers = ciphers.iter().map(|c| c.to_raw()).collect::<Vec<_>>();
        unsafe {
            cvt(SSLSetEnabledCiphers(self.0, ciphers.as_ptr(), ciphers.len() as size_t))
        }
    }

    pub fn negotiated_cipher(&self) -> Result<CipherSuite> {
        unsafe {
            let mut cipher = 0;
            try!(cvt(SSLGetNegotiatedCipher(self.0, &mut cipher)));
            Ok(CipherSuite::from_raw(cipher).unwrap())
        }
    }

    pub fn diffie_hellman_params(&self) -> Result<&[u8]> {
        unsafe {
            let mut ptr = ptr::null();
            let mut len = 0;
            try!(cvt(SSLGetDiffieHellmanParams(self.0, &mut ptr, &mut len)));
            Ok(slice::from_raw_parts(ptr as *const u8, len as usize))
        }
    }

    pub fn set_diffie_hellman_params(&mut self, dh_params: &[u8]) -> Result<()> {
        unsafe {
            cvt(SSLSetDiffieHellmanParams(self.0,
                                          dh_params.as_ptr() as *const _,
                                          dh_params.len() as size_t))
        }
    }

    pub fn peer_trust(&self) -> Result<SecTrust> {
        // Calling SSLCopyPeerTrust on an idle connection does not seem to be well defined,
        // so explicitly check for that
        if let SessionState::Idle = try!(self.state()) {
            try!(cvt(errSecBadReq));
        }

        unsafe {
            let mut trust = ptr::null_mut();
            try!(cvt(SSLCopyPeerTrust(self.0, &mut trust)));
            Ok(SecTrust::wrap_under_create_rule(trust))
        }
    }

    pub fn state(&self) -> Result<SessionState> {
        unsafe {
            let mut state = SSLSessionState::kSSLIdle;
            try!(cvt(SSLGetSessionState(self.0, &mut state)));
            Ok(SessionState::from_raw(state))
        }
    }

    pub fn handshake<S>(self, stream: S) -> result::Result<SslStream<S>, HandshakeError<S>>
            where S: Read + Write {
        unsafe {
            let ret = SSLSetIOFuncs(self.0, read_func::<S>, write_func::<S>);
            if ret != errSecSuccess {
                return Err(HandshakeError::Failure(Error::new(ret)));
            }

            let stream = Connection {
                stream: stream,
                err: None,
            };
            let stream = mem::transmute::<_, SSLConnectionRef>(Box::new(stream));
            let ret = SSLSetConnection(self.0, stream);
            if ret != errSecSuccess {
                let _conn = mem::transmute::<_, Box<Connection<S>>>(stream);
                return Err(HandshakeError::Failure(Error::new(ret)));
            }

            let stream = SslStream {
                ctx: self,
                _m: PhantomData,
            };

            match SSLHandshake(stream.ctx.0) {
                errSecSuccess => Ok(stream),
                errSSLPeerAuthCompleted => {
                    Err(HandshakeError::ServerAuthCompleted(MidHandshakeSslStream(stream)))
                }
                err => Err(HandshakeError::Failure(Error::new(err))),
            }
        }
    }
}

impl Drop for SslContext {
    #[cfg(not(feature = "OSX_10_8"))]
    fn drop(&mut self) {
        unsafe {
            SSLDisposeContext(self.0);
        }
    }

    #[cfg(feature = "OSX_10_8")]
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.as_CFTypeRef());
        }
    }
}

#[cfg(feature = "OSX_10_8")]
impl_TCFType!(SslContext, SSLContextRef, SSLContextGetTypeID);

struct Connection<S> {
    stream: S,
    err: Option<io::Error>,
}

// the logic here is based off of libcurl's

fn translate_err(e: &io::Error) -> OSStatus {
    match e.kind() {
        io::ErrorKind::NotFound => errSSLClosedGraceful,
        io::ErrorKind::ConnectionReset => errSSLClosedAbort,
        io::ErrorKind::WouldBlock => errSSLWouldBlock,
        _ => errSecIO,
    }
}

unsafe extern fn read_func<S: Read>(connection: SSLConnectionRef,
                                    data: *mut c_void,
                                    data_length: *mut size_t)
                                    -> OSStatus {
    let mut conn: &mut Connection<S> = mem::transmute(connection);
    let mut data = slice::from_raw_parts_mut(data as *mut u8, *data_length as usize);
    let mut start = 0;
    let mut ret = 0;

    while start < data.len() {
        match conn.stream.read(&mut data[start..]) {
            Ok(0) => {
                ret = errSSLClosedNoNotify;
                break;
            }
            Ok(len) => start += len,
            Err(e) => {
                ret = translate_err(&e);
                conn.err = Some(e);
                break;
            }
        }
    }

    *data_length = start as size_t;
    ret
}

unsafe extern fn write_func<S: Write>(connection: SSLConnectionRef,
                                      data: *const c_void,
                                      data_length: *mut size_t)
                                      -> OSStatus {
    let mut conn: &mut Connection<S> = mem::transmute(connection);
    let data = slice::from_raw_parts(data as *mut u8, *data_length as usize);
    let mut start = 0;
    let mut ret = 0;

    while start < data.len() {
        match conn.stream.write(&data[start..]) {
            Ok(0) => {
                ret = errSSLClosedNoNotify;
                break;
            },
            Ok(len) => start += len,
            Err(e) => {
                ret = translate_err(&e);
                conn.err = Some(e);
                break;
            }
        }
    }

    *data_length = start as size_t;
    ret
}

pub struct SslStream<S> {
    ctx: SslContext,
    _m: PhantomData<S>,
}

impl<S: fmt::Debug> fmt::Debug for SslStream<S> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("SslStream")
            .field("ctx", &self.ctx)
            .field("stream", self.get_ref())
            .finish()
    }
}

impl<S> Drop for SslStream<S> {
    fn drop(&mut self) {
        unsafe {
            SSLClose(self.ctx.0);

            let mut conn = ptr::null();
            let ret = SSLGetConnection(self.ctx.0, &mut conn);
            assert!(ret == errSecSuccess);
            mem::transmute::<_, Box<Connection<S>>>(conn);
        }
    }
}

impl<S> SslStream<S> {
    pub fn get_ref(&self) -> &S {
        &self.connection().stream
    }

    pub fn get_mut(&mut self) -> &mut S {
        &mut self.connection_mut().stream
    }

    pub fn context(&self) -> &SslContext {
        &self.ctx
    }

    fn connection(&self) -> &Connection<S> {
        unsafe {
            let mut conn = ptr::null();
            let ret = SSLGetConnection(self.ctx.0, &mut conn);
            assert!(ret == errSecSuccess);

            mem::transmute(conn)
        }
    }

    fn connection_mut(&mut self) -> &mut Connection<S> {
        unsafe {
            let mut conn = ptr::null();
            let ret = SSLGetConnection(self.ctx.0, &mut conn);
            assert!(ret == errSecSuccess);

            mem::transmute(conn)
        }
    }

    fn get_error(&mut self, ret: OSStatus) -> io::Error {
        let conn = self.connection_mut();
        if let Some(err) = conn.err.take() {
            err
        } else {
            io::Error::new(io::ErrorKind::Other, Error::new(ret))
        }
    }
}

impl<S: Read + Write> Read for SslStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let mut nread = 0;
            let ret = SSLRead(self.ctx.0,
                              buf.as_mut_ptr() as *mut _,
                              buf.len() as size_t,
                              &mut nread);
            match ret {
                errSecSuccess => Ok(nread as usize),
                errSSLClosedGraceful
                    | errSSLClosedAbort
                    | errSSLClosedNoNotify => Ok(0),
                _ => Err(self.get_error(ret)),
            }
        }
    }
}

impl<S: Read + Write> Write for SslStream<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let mut nwritten = 0;
            let ret = SSLWrite(self.ctx.0,
                               buf.as_ptr() as *const _,
                               buf.len() as size_t,
                               &mut nwritten);
            if ret == errSecSuccess {
                Ok(nwritten as usize)
            } else {
                Err(self.get_error(ret))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.connection_mut().stream.flush()
    }
}

#[cfg(test)]
mod test {
    use std::io::prelude::*;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    use super::*;
    use cipher_suite::CipherSuite;
    use test::{certificate, identity};

    #[test]
    fn connect() {
        let mut ctx = p!(SslContext::new(ProtocolSide::Client));
        p!(ctx.set_peer_domain_name("google.com"));
        let stream = p!(TcpStream::connect("google.com:443"));
        p!(ctx.handshake(stream));
    }

    #[test]
    fn connect_bad_domain() {
        let mut ctx = p!(SslContext::new(ProtocolSide::Client));
        p!(ctx.set_peer_domain_name("foobar.com"));
        let stream = p!(TcpStream::connect("google.com:443"));
        match ctx.handshake(stream) {
            Ok(_) => panic!("expected failure"),
            Err(_) => {}
        }
    }

    #[test]
    fn load_page() {
        let mut ctx = p!(SslContext::new(ProtocolSide::Client));
        p!(ctx.set_peer_domain_name("google.com"));
        let stream = p!(TcpStream::connect("google.com:443"));
        let mut stream = p!(ctx.handshake(stream));
        p!(stream.write_all(b"GET / HTTP/1.0\r\n\r\n"));
        p!(stream.flush());
        let mut buf = String::new();
        p!(stream.read_to_string(&mut buf));
        assert!(buf.starts_with("HTTP/1.0 200 OK"));
        assert!(buf.ends_with("</html>"));
    }

    #[test]
    fn server_client() {
        let listener = p!(TcpListener::bind("localhost:15410"));

        let handle = thread::spawn(move || {
            let mut ctx = p!(SslContext::new(ProtocolSide::Server));
            let identity = identity();
            p!(ctx.set_certificate(&identity, &[]));

            let stream = p!(listener.accept()).0;
            let mut stream = p!(ctx.handshake(stream));

            let mut buf = [0; 12];
            p!(stream.read(&mut buf));
            assert_eq!(&buf[..], b"hello world!");
        });

        let mut ctx = p!(SslContext::new(ProtocolSide::Client));
        p!(ctx.set_break_on_server_auth(true));
        let stream = p!(TcpStream::connect("localhost:15410"));

        let stream = match ctx.handshake(stream) {
            Ok(_) => panic!("unexpected success"),
            Err(HandshakeError::ServerAuthCompleted(stream)) => stream,
            Err(HandshakeError::Failure(err)) => panic!("unexpected error {}", err),
        };

        let mut peer_trust = p!(stream.context().peer_trust());
        p!(peer_trust.set_anchor_certificates(&[certificate()]));
        let result = p!(peer_trust.evaluate());
        assert!(result.success());

        let mut stream = p!(stream.handshake());
        p!(stream.write_all(b"hello world!"));

        handle.join().unwrap();
    }

    #[test]
    fn idle_context_peer_trust() {
        let ctx = p!(SslContext::new(ProtocolSide::Server));
        assert!(ctx.peer_trust().is_err());
    }

    #[test]
    fn cipher_configuration() {
        let mut ctx = p!(SslContext::new(ProtocolSide::Server));
        let ciphers = p!(ctx.enabled_ciphers());
        let ciphers = ciphers.iter()
            .enumerate()
            .filter_map(|(i, c)| if i % 2 == 0 { Some(*c) } else { None })
            .collect::<Vec<_>>();
        p!(ctx.set_enabled_ciphers(&ciphers));
        assert_eq!(ciphers, p!(ctx.enabled_ciphers()));
    }

    #[test]
    fn negotiated_cipher() {
        let listener = p!(TcpListener::bind("localhost:15411"));

        let handle = thread::spawn(move || {
            let mut ctx = p!(SslContext::new(ProtocolSide::Server));
            let identity = identity();
            p!(ctx.set_certificate(&identity, &[]));
            p!(ctx.set_enabled_ciphers(&[CipherSuite::TLS_DHE_RSA_WITH_AES_256_CBC_SHA256,
                                         CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA256]));

            let stream = p!(listener.accept()).0;
            let mut stream = p!(ctx.handshake(stream));
            assert_eq!(CipherSuite::TLS_DHE_RSA_WITH_AES_256_CBC_SHA256,
                       p!(stream.context().negotiated_cipher()));
            let mut buf = [0; 1];
            p!(stream.read(&mut buf));
        });

        let mut ctx = p!(SslContext::new(ProtocolSide::Client));
        p!(ctx.set_break_on_server_auth(true));
        p!(ctx.set_enabled_ciphers(&[CipherSuite::TLS_DHE_PSK_WITH_AES_128_CBC_SHA256,
                                     CipherSuite::TLS_DHE_RSA_WITH_AES_256_CBC_SHA256]));
        let stream = p!(TcpStream::connect("localhost:15411"));

        let stream = match ctx.handshake(stream) {
            Ok(_) => panic!("unexpected success"),
            Err(HandshakeError::ServerAuthCompleted(stream)) => stream,
            Err(HandshakeError::Failure(err)) => panic!("unexpected error {}", err),
        };

        let mut stream = p!(stream.handshake());
        assert_eq!(CipherSuite::TLS_DHE_RSA_WITH_AES_256_CBC_SHA256,
                   p!(stream.context().negotiated_cipher()));
        p!(stream.write(&[0]));

        handle.join().unwrap();
    }
}
