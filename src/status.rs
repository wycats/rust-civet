pub use self::StatusCode::*;

#[derive(Copy)]
pub enum StatusCode {
    Informational(uint, &'static str),
    Continue,
    SwitchingProtocols,
    Successful(uint, &'static str),
    OK,
    Created,
    Accepted,
    NonAuthoritativeInformation,
    NoContent,
    ResetContent,
    PartialContent,
    Redirection(uint, &'static str),
    MultipleChoices,
    MovedPermanently,
    Found,
    SeeOther,
    NotModified,
    UseProxy,
    TemporaryRedirect,
    ClientError(uint, &'static str),
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    NotAcceptable,
    ProxyAuthenticationRequired,
    RequestTimeout,
    Conflict,
    Gone,
    LengthRequired,
    PreconditionFailed,
    RequestEntityTooLarge,
    RequestUriTooLong,
    UnsupportedMediaType,
    RequestedRangeNotSatisfiable,
    ExpectationFailed,
    ServerError(uint, &'static str),
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceUnavailable,
    GatewayTimeout,
    HttpVersionNotSupported,
    Other(uint, &'static str),
}

impl StatusCode {
    pub fn to_code(&self) -> (uint, &'static str) {
        match *self {
            Informational(num, string) |
                Successful(num, string) |
                Redirection(num, string) |
                ClientError(num, string) |
                ServerError(num, string) => (num, string),
            Continue => (100, "Continue"),
            SwitchingProtocols => (101, "Switching Protocols"),
            OK => (200, "OK"),
            Created => (201, "Created"),
            Accepted => (202, "Accepted"),
            NonAuthoritativeInformation => (203, "Non-Authoritative Information"),
            NoContent => (204, "No Content"),
            ResetContent => (205, "Reset Content"),
            PartialContent => (206, "Partial Content"),
            MultipleChoices => (300, "Multiple Choices"),
            MovedPermanently => (301, "Moved Permanently"),
            Found => (302, "Found"),
            SeeOther => (303, "See Other"),
            NotModified => (304, "Not Modified"),
            UseProxy => (305, "Use Proxy"),
            TemporaryRedirect => (307, "Temporary Redirect"),
            BadRequest => (400, "Bad Request"),
            Unauthorized => (401, "Unauthorized"),
            Forbidden => (403, "Forbidden"),
            NotFound => (404, "Not Found"),
            MethodNotAllowed => (405, "Method Not Allowed"),
            NotAcceptable => (406, "Not Acceptable"),
            ProxyAuthenticationRequired => (407, "Proxy Authentication Required"),
            RequestTimeout => (408, "Request Timeout"),
            Conflict => (409, "Conflict"),
            Gone => (410, "Gone"),
            LengthRequired => (411, "Length Required"),
            PreconditionFailed => (412, "Precondition Failed"),
            RequestEntityTooLarge => (413, "Request Entity Too Large"),
            RequestUriTooLong => (414, "Request-URI Too Long"),
            UnsupportedMediaType => (415, "Unsupported Media Type"),
            RequestedRangeNotSatisfiable => (416, "Requested Range Not Satisfiable"),
            ExpectationFailed => (417, "Expectation Failed"),
            InternalServerError => (500, "Internal Server Error"),
            NotImplemented => (501, "Not Implemented"),
            BadGateway => (502, "Bad Gateway"),
            ServiceUnavailable => (503, "Service Unavailable"),
            GatewayTimeout => (504, "Gateway Timeout"),
            HttpVersionNotSupported => (505, "HTTP Version Not Supported"),
            Other(n, s) => (n, s),
        }
    }
}

pub trait ToStatusCode {
    fn to_status(&self) -> Result<StatusCode, ()>;
}

impl ToStatusCode for StatusCode {
    fn to_status(&self) -> Result<StatusCode, ()> {
        Ok(*self)
    }
}

impl ToStatusCode for (uint, &'static str) {
    fn to_status(&self) -> Result<StatusCode, ()> {
        let (code, name) = *self;
        Ok(Other(code, name))
    }
}

impl ToStatusCode for int {
    fn to_status(&self) -> Result<StatusCode, ()> {
        (*self as uint).to_status()
    }
}

impl ToStatusCode for uint {
    fn to_status(&self) -> Result<StatusCode, ()> {
        match *self {
            num @ 102 ... 199 => Ok(Informational(num, "Informational")),
            num @ 207 ... 299 => Ok(Successful(num, "Successful")),
            num @ 306 | num @ 308 ... 399 => Ok(Redirection(num, "Redirection")),
            num @ 402 | num @ 418 ... 499 => Ok(ClientError(num, "Client Error")),
            num @ 506 ... 599 => Ok(ServerError(num, "Server Error")),
            100 => Ok(Continue),
            101 => Ok(SwitchingProtocols),
            200 => Ok(OK),
            201 => Ok(Created),
            202 => Ok(Accepted),
            203 => Ok(NonAuthoritativeInformation),
            204 => Ok(NoContent),
            205 => Ok(ResetContent),
            206 => Ok(PartialContent),
            300 => Ok(MultipleChoices),
            301 => Ok(MovedPermanently),
            302 => Ok(Found),
            303 => Ok(SeeOther),
            304 => Ok(NotModified),
            305 => Ok(UseProxy),
            307 => Ok(TemporaryRedirect),
            400 => Ok(BadRequest),
            401 => Ok(Unauthorized),
            403 => Ok(Forbidden),
            404 => Ok(NotFound),
            405 => Ok(MethodNotAllowed),
            406 => Ok(NotAcceptable),
            407 => Ok(ProxyAuthenticationRequired),
            408 => Ok(RequestTimeout),
            409 => Ok(Conflict),
            410 => Ok(Gone),
            411 => Ok(LengthRequired),
            412 => Ok(PreconditionFailed),
            413 => Ok(RequestEntityTooLarge),
            414 => Ok(RequestUriTooLong),
            415 => Ok(UnsupportedMediaType),
            416 => Ok(RequestedRangeNotSatisfiable),
            417 => Ok(ExpectationFailed),
            500 => Ok(InternalServerError),
            501 => Ok(NotImplemented),
            502 => Ok(BadGateway),
            503 => Ok(ServiceUnavailable),
            504 => Ok(GatewayTimeout),
            505 => Ok(HttpVersionNotSupported),
            _ => Err(())
        }
    }
}

impl ToStatusCode for () {
    fn to_status(&self) -> Result<StatusCode, ()> {
        Err(())
    }
}
