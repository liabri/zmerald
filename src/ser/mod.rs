use serde::{ ser, Deserialize, Serialize };
use std::io;

use crate::{
    error::{ Error, Result },
    parse::{ is_ident_first_char, is_ident_other_char, LargeSInt, LargeUInt },
};

mod value;

pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where W: io::Write, T: ?Sized + ser::Serialize {
    let mut s = Serializer::with_options(writer, None)?;
    value.serialize(&mut s)
}

pub fn to_writer_pretty<W, T>(writer: W, value: &T, config: PrettyConfig) -> Result<()>
where W: io::Write, T: ?Sized + ser::Serialize {
    let mut s = Serializer::with_options(writer, Some(config), )?;
    value.serialize(&mut s)
}

pub fn to_string<T>(value: &T) -> Result<String>
where T: ?Sized + ser::Serialize {
    let mut output = Vec::new();
    let mut s = Serializer::with_options(&mut output, None, )?;
    value.serialize(&mut s)?;
    Ok(String::from_utf8(output).expect("Ron should be utf-8"))
}

pub fn to_string_pretty<T>(value: &T, config: PrettyConfig) -> Result<String>
where T: ?Sized + ser::Serialize {
    let mut output = Vec::new();
    let mut s = Serializer::with_options(&mut output, Some(config), )?;
    value.serialize(&mut s)?;
    Ok(String::from_utf8(output).expect("Ron should be utf-8"))
}

struct Pretty {
    indent: usize,
    sequence_index: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct PrettyConfig {
    pub depth_limit: usize,
    pub new_line: String,
    pub indentor: String,
    pub separator: String,
    // Whether to emit struct names
    pub struct_names: bool,
    pub separate_tuple_members: bool,
    pub enumerate_arrays: bool,
    pub decimal_floats: bool,
    pub compact_arrays: bool,
}

impl PrettyConfig {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn depth_limit(mut self, depth_limit: usize) -> Self {
        self.depth_limit = depth_limit;

        self
    }

    pub fn new_line(mut self, new_line: String) -> Self {
        self.new_line = new_line;

        self
    }

    pub fn indentor(mut self, indentor: String) -> Self {
        self.indentor = indentor;

        self
    }

    pub fn separator(mut self, separator: String) -> Self {
        self.separator = separator;

        self
    }

    pub fn struct_names(mut self, struct_names: bool) -> Self {
        self.struct_names = struct_names;

        self
    }

    pub fn separate_tuple_members(mut self, separate_tuple_members: bool) -> Self {
        self.separate_tuple_members = separate_tuple_members;

        self
    }

    pub fn enumerate_arrays(mut self, enumerate_arrays: bool) -> Self {
        self.enumerate_arrays = enumerate_arrays;

        self
    }

    pub fn decimal_floats(mut self, decimal_floats: bool) -> Self {
        self.decimal_floats = decimal_floats;

        self
    }

    pub fn compact_arrays(mut self, compact_arrays: bool) -> Self {
        self.compact_arrays = compact_arrays;

        self
    }
}

impl Default for PrettyConfig {
    fn default() -> Self {
        PrettyConfig {
            depth_limit: !0,
            new_line: String::from("\n"),
            indentor: String::from("    "),
            separator: String::from(" "),
            struct_names: false,
            separate_tuple_members: false,
            enumerate_arrays: false,
            decimal_floats: false,
            compact_arrays: false,
        }
    }
}

pub struct Serializer<W: io::Write> {
    output: W,
    pretty: Option<(PrettyConfig, Pretty)>,
    is_empty: Option<bool>,
    newtype_variant: bool,
}

impl<W: io::Write> Serializer<W> {
    pub fn new(writer: W, config: Option<PrettyConfig>) -> Result<Self> {
        Self::with_options(writer, config)
    }

    pub fn with_options(writer: W, config: Option<PrettyConfig>) -> Result<Self> {
        Ok(Serializer {
            output: writer,
            pretty: config.map(|conf| {(
                conf,
                Pretty {
                    indent: 0,
                    sequence_index: Vec::new(),
                })
            }),
            is_empty: None,
            newtype_variant: true,
        })
    }

    fn separate_tuple_members(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(false, |&(ref config, _)| config.separate_tuple_members)
    }

    fn decimal_floats(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(false, |&(ref config, _)| config.decimal_floats)
    }

    fn compact_arrays(&self) -> bool {
        self.pretty
            .as_ref()
            .map_or(false, |&(ref config, _)| config.compact_arrays)
    }

    fn start_indent(&mut self) -> Result<()> {
        if let Some((ref config, ref mut pretty)) = self.pretty {
            pretty.indent += 1;
            if pretty.indent <= config.depth_limit {
                let is_empty = self.is_empty.unwrap_or(false);

                if !is_empty {
                    self.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        Ok(())
    }

    fn indent(&mut self) -> io::Result<()> {
        if let Some((ref config, ref pretty)) = self.pretty {
            if pretty.indent <= config.depth_limit {
                for _ in 0..pretty.indent {
                    self.output.write_all(config.indentor.as_bytes())?;
                }
            }
        }
        Ok(())
    }

    fn end_indent(&mut self) -> io::Result<()> {
        if let Some((ref config, ref mut pretty)) = self.pretty {
            if pretty.indent <= config.depth_limit {
                let is_empty = self.is_empty.unwrap_or(false);

                if !is_empty {
                    for _ in 1..pretty.indent {
                        self.output.write_all(config.indentor.as_bytes())?;
                    }
                }
            }
            pretty.indent -= 1;

            self.is_empty = None;
        }
        Ok(())
    }

    fn serialize_escaped_str(&mut self, value: &str) -> io::Result<()> {
        self.output.write_all(b"\"")?;
        let mut scalar = [0u8; 4];
        for c in value.chars().flat_map(|c| c.escape_debug()) {
            self.output
                .write_all(c.encode_utf8(&mut scalar).as_bytes())?;
        }
        self.output.write_all(b"\"")?;
        Ok(())
    }

    fn serialize_sint(&mut self, value: impl Into<LargeSInt>) -> Result<()> {
        // TODO optimize
        write!(self.output, "{}", value.into())?;

        Ok(())
    }

    fn serialize_uint(&mut self, value: impl Into<LargeUInt>) -> Result<()> {
        // TODO optimize
        write!(self.output, "{}", value.into())?;

        Ok(())
    }

    fn write_identifier(&mut self, name: &str) -> io::Result<()> {
        let mut bytes = name.as_bytes().iter().cloned();
        if !bytes.next().map_or(false, is_ident_first_char) || !bytes.all(is_ident_other_char) {
            self.output.write_all(b"r#")?;
        }
        self.output.write_all(name.as_bytes())?;
        Ok(())
    }

    fn struct_names(&self) -> bool {
        self.pretty
            .as_ref()
            .map(|(pc, _)| pc.struct_names)
            .unwrap_or(false)
    }
}

impl<'a, W: io::Write> ser::Serializer for &'a mut Serializer<W> {
    type Error = Error;
    type Ok = ();
    type SerializeMap = Compound<'a, W>;
    type SerializeSeq = Compound<'a, W>;
    type SerializeStruct = Compound<'a, W>;
    type SerializeStructVariant = Compound<'a, W>;
    type SerializeTuple = Compound<'a, W>;
    type SerializeTupleStruct = Compound<'a, W>;
    type SerializeTupleVariant = Compound<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output.write_all(if v { b"true" } else { b"false" })?;
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_sint(v)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_sint(v)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_sint(v)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.serialize_sint(v)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_uint(v)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_uint(v)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_uint(v)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.serialize_uint(v)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        write!(self.output, "{}", v)?;
        if self.decimal_floats() && (v - v.floor()).abs() < f32::EPSILON {
            write!(self.output, ".0")?;
        }
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        write!(self.output, "{}", v)?;
        if self.decimal_floats() && (v - v.floor()).abs() < f64::EPSILON {
            write!(self.output, ".0")?;
        }
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.output.write_all(b"'")?;
        if v == '\\' || v == '\'' {
            self.output.write_all(b"\\")?;
        }
        write!(self.output, "{}", v)?;
        self.output.write_all(b"'")?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_escaped_str(v)?;

        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.serialize_str(base64::encode(v).as_str())
    }

    fn serialize_none(self) -> Result<()> {
        self.output.write_all(b"None")?;

        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where T: ?Sized + Serialize {
        value.serialize(&mut *self)?;
        Ok(())
    }

    fn serialize_unit(self) -> Result<()> {
        if !self.newtype_variant {
            self.output.write_all(b"()")?;
        }

        self.newtype_variant = false;

        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        if self.struct_names() && !self.newtype_variant {
            self.write_identifier(name)?;

            Ok(())
        } else {
            self.serialize_unit()
        }
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, variant: &'static str) -> Result<()> {
        self.write_identifier(variant)?;

        Ok(())
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<()>
    where T: ?Sized + Serialize {
        if self.struct_names() {
            self.write_identifier(name)?;
        }

        self.output.write_all(b"(")?;
        value.serialize(&mut *self)?;
        self.output.write_all(b")")?;
        Ok(())
    }

    fn serialize_newtype_variant<T>(self, _: &'static str, _: u32, variant: &'static str, value: &T) -> Result<()>
    where T: ?Sized + Serialize {
        self.write_identifier(variant)?;
        self.output.write_all(b"(")?;

        self.newtype_variant = true;

        value.serialize(&mut *self)?;

        self.newtype_variant = false;

        self.output.write_all(b")")?;
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.newtype_variant = false;

        self.output.write_all(b"[")?;

        if let Some(len) = len {
            self.is_empty = Some(len == 0);
        }

        if !self.compact_arrays() {
            self.start_indent()?;
        }

        if let Some((_, ref mut pretty)) = self.pretty {
            pretty.sequence_index.push(0);
        }

        Ok(Compound {
            ser: self,
            state: State::First,
            newtype_variant: false,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        let old_newtype_variant = self.newtype_variant;
        self.newtype_variant = false;

        if !old_newtype_variant {
            self.output.write_all(b"(")?;
        }

        if self.separate_tuple_members() {
            self.is_empty = Some(len == 0);

            self.start_indent()?;
        }

        Ok(Compound {
            ser: self,
            state: State::First,
            newtype_variant: old_newtype_variant,
        })
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct> {
        if self.struct_names() && !self.newtype_variant {
            self.write_identifier(name)?;
        }

        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(self, _: &'static str, _: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant> {
        self.newtype_variant = false;

        self.write_identifier(variant)?;
        self.output.write_all(b"(")?;

        if self.separate_tuple_members() {
            self.is_empty = Some(len == 0);

            self.start_indent()?;
        }

        Ok(Compound {
            ser: self,
            state: State::First,
            newtype_variant: false,
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        self.newtype_variant = false;

        self.output.write_all(b"{")?;

        if let Some(len) = len {
            self.is_empty = Some(len == 0);
        }

        self.start_indent()?;

        Ok(Compound {
            ser: self,
            state: State::First,
            newtype_variant: false,
        })
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        let old_newtype_variant = self.newtype_variant;
        self.newtype_variant = false;

        if !old_newtype_variant {
            if self.struct_names() {
                self.write_identifier(name)?;
            }
            self.output.write_all(b"(")?;
        }

        self.is_empty = Some(len == 0);
        self.start_indent()?;

        Ok(Compound {
            ser: self,
            state: State::First,
            newtype_variant: old_newtype_variant,
        })
    }

    fn serialize_struct_variant(self, _: &'static str, _: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant> {
        self.newtype_variant = false;

        self.write_identifier(variant)?;
        self.output.write_all(b"(")?;

        self.is_empty = Some(len == 0);
        self.start_indent()?;

        Ok(Compound {
            ser: self,
            state: State::First,
            newtype_variant: false,
        })
    }
}

enum State {
    First,
    Rest,
}

#[doc(hidden)]
pub struct Compound<'a, W: io::Write> {
    ser: &'a mut Serializer<W>,
    state: State,
    newtype_variant: bool,
}

impl<'a, W: io::Write> ser::SerializeSeq for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where T: ?Sized + Serialize {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_all(b",")?;
            if let Some((ref config, ref mut pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit && !config.compact_arrays {
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                } else {
                    self.ser.output.write_all(config.separator.as_bytes())?;
                }
            }
        }

        if !self.ser.compact_arrays() {
            self.ser.indent()?;
        }

        if let Some((ref mut config, ref mut pretty)) = self.ser.pretty {
            if pretty.indent <= config.depth_limit && config.enumerate_arrays {
                let index = pretty.sequence_index.last_mut().unwrap();
                write!(self.ser.output, "/*[{}]*/ ", index)?;
                *index += 1;
            }
        }

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref mut pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit && !config.compact_arrays {
                    self.ser.output.write_all(b",")?;
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }

        if !self.ser.compact_arrays() {
            self.ser.end_indent()?;
        }

        if let Some((_, ref mut pretty)) = self.ser.pretty {
            pretty.sequence_index.pop();
        }

        // seq always disables `self.newtype_variant`
        self.ser.output.write_all(b"]")?;
        Ok(())
    }
}

impl<'a, W: io::Write> ser::SerializeTuple for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where T: ?Sized + Serialize {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_all(b",")?;
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit && self.ser.separate_tuple_members() {
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                } else {
                    self.ser.output.write_all(config.separator.as_bytes())?;
                }
            }
        }

        if self.ser.separate_tuple_members() {
            self.ser.indent()?;
        }

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if self.ser.separate_tuple_members() && pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(b",")?;
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        if self.ser.separate_tuple_members() {
            self.ser.end_indent()?;
        }

        if !self.newtype_variant {
            self.ser.output.write_all(b")")?;
        }

        Ok(())
    }
}

// Same thing but for tuple structs.
impl<'a, W: io::Write> ser::SerializeTupleStruct for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where T: ?Sized + Serialize {
        ser::SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeTuple::end(self)
    }
}

impl<'a, W: io::Write> ser::SerializeTupleVariant for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where T: ?Sized + Serialize {
        ser::SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeTuple::end(self)
    }
}

impl<'a, W: io::Write> ser::SerializeMap for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where T: ?Sized + Serialize {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_all(b",")?;

            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                } else {
                    self.ser.output.write_all(config.separator.as_bytes())?;
                }
            }
        }
        self.ser.indent()?;
        key.serialize(&mut *self.ser)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where T: ?Sized + Serialize {
        self.ser.output.write_all(b":")?;

        if let Some((ref config, _)) = self.ser.pretty {
            self.ser.output.write_all(config.separator.as_bytes())?;
        }

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(b",")?;
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        self.ser.end_indent()?;
        // map always disables `self.newtype_variant`
        self.ser.output.write_all(b"}")?;
        Ok(())
    }
}

impl<'a, W: io::Write> ser::SerializeStruct for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where T: ?Sized + Serialize {
        if let State::First = self.state {
            self.state = State::Rest;
        } else {
            self.ser.output.write_all(b",")?;

            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                } else {
                    self.ser.output.write_all(config.separator.as_bytes())?;
                }
            }
        }
        self.ser.indent()?;
        self.ser.write_identifier(key)?;
        self.ser.output.write_all(b":")?;

        if let Some((ref config, _)) = self.ser.pretty {
            self.ser.output.write_all(config.separator.as_bytes())?;
        }

        value.serialize(&mut *self.ser)?;

        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::Rest = self.state {
            if let Some((ref config, ref pretty)) = self.ser.pretty {
                if pretty.indent <= config.depth_limit {
                    self.ser.output.write_all(b",")?;
                    self.ser.output.write_all(config.new_line.as_bytes())?;
                }
            }
        }
        self.ser.end_indent()?;
        if !self.newtype_variant {
            self.ser.output.write_all(b")")?;
        }
        Ok(())
    }
}

impl<'a, W: io::Write> ser::SerializeStructVariant for Compound<'a, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where T: ?Sized + Serialize {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<()> {
        ser::SerializeStruct::end(self)
    }
}
