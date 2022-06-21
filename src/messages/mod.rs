use std::io;

use bytes::{BufMut, BytesMut};

pub(crate) trait Message: Sized {
    /// Return the type code of the message. In order to maintain backward
    /// compatibility, `Startup` has no message type.
    #[inline]
    fn message_type() -> Option<u8> {
        None
    }

    /// Return the length of the message, including the length integer itself.
    fn message_length(&self) -> usize;

    fn encode_body(&self, buf: &mut BytesMut) -> io::Result<()>;

    fn decode_body(buf: &mut BytesMut) -> io::Result<Self>;

    fn encode(&self, buf: &mut BytesMut) -> io::Result<()> {
        if let Some(mt) = Self::message_type() {
            buf.put_u8(mt);
        }

        buf.put_i32(self.message_length() as i32);
        self.encode_body(buf)
    }

    fn decode(buf: &mut BytesMut) -> io::Result<Option<Self>> {
        if let Some(mt) = Self::message_type() {
            codec::get_and_ensure_message_type(buf, mt)?;
        }

        codec::decode_packet(buf, |buf, _| Self::decode_body(buf))
    }
}

mod codec;
mod response;
mod result;
mod simplequery;
mod startup;
mod terminate;

pub enum PgWireMessage {
    Startup(startup::Startup),
    Authentication(startup::Authentication),
    Password(startup::Password),
}

#[cfg(test)]
mod test {
    use super::response::*;
    use super::result::*;
    use super::simplequery::*;
    use super::startup::*;
    use super::terminate::*;
    use super::Message;
    use bytes::{Buf, BytesMut};

    macro_rules! roundtrip {
        ($ins:ident, $st:ty) => {
            let mut buffer = BytesMut::new();
            $ins.encode(&mut buffer).unwrap();

            assert!(buffer.remaining() > 0);

            let item2 = <$st>::decode(&mut buffer).unwrap().unwrap();

            assert_eq!(buffer.remaining(), 0);
            assert_eq!($ins, item2);
        };
    }

    #[test]
    fn test_startup() {
        let mut s = Startup::default();
        s.parameters_mut()
            .insert("user".to_owned(), "tomcat".to_owned());

        roundtrip!(s, Startup);
    }

    #[test]
    fn test_authentication() {
        let ss = vec![
            Authentication::Ok,
            Authentication::CleartextPassword,
            Authentication::KerberosV5,
        ];
        for s in ss {
            roundtrip!(s, Authentication);
        }

        let md5pass = Authentication::MD5Password([b'p', b's', b't', b'g']);
        roundtrip!(md5pass, Authentication);
    }

    #[test]
    fn test_password() {
        let s = Password::new("pgwire".to_owned());
        roundtrip!(s, Password);
    }

    #[test]
    fn test_parameter_status() {
        let pps = ParameterStatus::new("cli".to_owned(), "psql".to_owned());
        roundtrip!(pps, ParameterStatus);
    }

    #[test]
    fn test_query() {
        let query = Query::new("SELECT 1".to_owned());
        roundtrip!(query, Query);
    }

    #[test]
    fn test_command_complete() {
        let cc = CommandComplete::new("DELETE 5".to_owned());
        roundtrip!(cc, CommandComplete);
    }

    #[test]
    fn test_ready_for_query() {
        let r4q = ReadyForQuery::new(b'I');
        roundtrip!(r4q, ReadyForQuery);
    }

    #[test]
    fn test_error_response() {
        let mut error = ErrorResponse::default();
        error.fields_mut().push((b'R', "ERROR".to_owned()));
        error.fields_mut().push((b'K', "cli".to_owned()));

        roundtrip!(error, ErrorResponse);
    }

    #[test]
    fn test_row_description() {
        let mut row_description = RowDescription::default();

        let mut f1 = FieldDescription::default();
        f1.set_name("id".into());
        f1.set_table_id(1001);
        f1.set_column_id(10001);
        f1.set_type_id(1083);
        f1.set_type_size(4);
        f1.set_type_modifier(-1);
        f1.set_format_code(FORMAT_CODE_TEXT);
        row_description.fields_mut().push(f1);

        let mut f2 = FieldDescription::default();
        f2.set_name("name".into());
        f2.set_table_id(1001);
        f2.set_column_id(10001);
        f2.set_type_id(1099);
        f2.set_type_size(-1);
        f2.set_type_modifier(-1);
        f2.set_format_code(FORMAT_CODE_TEXT);
        row_description.fields_mut().push(f2);

        roundtrip!(row_description, RowDescription);
    }

    #[test]
    fn test_data_row() {
        let mut row0 = DataRow::new();
        row0.fields_mut().push(Some(vec![b'1']));
        row0.fields_mut().push(None);

        let mut row1 = DataRow::new();
        row1.fields_mut().push(Some(vec![b'2']));
        row1.fields_mut().push(Some(vec![b't', b'o', b'm']));

        roundtrip!(row0, DataRow);
        roundtrip!(row1, DataRow);
    }

    #[test]
    fn test_terminate() {
        let terminate = Terminate::new();
        roundtrip!(terminate, Terminate);
    }
}
