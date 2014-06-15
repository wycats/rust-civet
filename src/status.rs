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
    HttpVersionNotSupported
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
            HttpVersionNotSupported => (505, "HTTP Version Not Supported")
        }
    }
}

pub trait ToStatusCode {
    fn to_status(&self) -> Result<(uint, &'static str), ()>;
}

impl ToStatusCode for StatusCode {
    fn to_status(&self) -> Result<(uint, &'static str), ()> {
        Ok(self.to_code())
    }
}

impl ToStatusCode for (uint, &'static str) {
    fn to_status(&self) -> Result<(uint, &'static str), ()> {
        Ok(*self)
    }
}

impl ToStatusCode for int {
    fn to_status(&self) -> Result<(uint, &'static str), ()> {
        (*self as uint).to_status()
    }
}

impl ToStatusCode for uint {
    fn to_status(&self) -> Result<(uint, &'static str), ()> {
        match *self {
            num @ 102 .. 199 => Ok((num, "Informational")),
            num @ 207 .. 299 => Ok((num, "Successful")),
            num @ 306 | num @ 308 .. 399 => Ok((num, "Redirection")),
            num @ 402 | num @ 418 .. 499 => Ok((num, "Client Error")),
            num @ 506 .. 599 => Ok((num, "Server Error")),
            100 => Continue.to_status(),
            101 => SwitchingProtocols.to_status(),
            200 => OK.to_status(),
            201 => Created.to_status(),
            202 => Accepted.to_status(),
            203 => NonAuthoritativeInformation.to_status(),
            204 => NoContent.to_status(),
            205 => ResetContent.to_status(),
            206 => PartialContent.to_status(),
            300 => MultipleChoices.to_status(),
            301 => MovedPermanently.to_status(),
            302 => Found.to_status(),
            303 => SeeOther.to_status(),
            304 => NotModified.to_status(),
            305 => UseProxy.to_status(),
            307 => TemporaryRedirect.to_status(),
            400 => BadRequest.to_status(),
            401 => Unauthorized.to_status(),
            403 => Forbidden.to_status(),
            404 => NotFound.to_status(),
            405 => MethodNotAllowed.to_status(),
            406 => NotAcceptable.to_status(),
            407 => ProxyAuthenticationRequired.to_status(),
            408 => RequestTimeout.to_status(),
            409 => Conflict.to_status(),
            410 => Gone.to_status(),
            411 => LengthRequired.to_status(),
            412 => PreconditionFailed.to_status(),
            413 => RequestEntityTooLarge.to_status(),
            414 => RequestUriTooLong.to_status(),
            415 => UnsupportedMediaType.to_status(),
            416 => RequestedRangeNotSatisfiable.to_status(),
            417 => ExpectationFailed.to_status(),
            500 => InternalServerError.to_status(),
            501 => NotImplemented.to_status(),
            502 => BadGateway.to_status(),
            503 => ServiceUnavailable.to_status(),
            504 => GatewayTimeout.to_status(),
            505 => HttpVersionNotSupported.to_status(),
            _ => Err(())
        }
    }
}

impl ToStatusCode for () {
    fn to_status(&self) -> Result<(uint, &'static str), ()> {
        Err(())
    }
}
