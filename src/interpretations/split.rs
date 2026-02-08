use linkme::distributed_slice;

use crate::{
    api::*,
    implement_try_from_xell,
    utils::{
        ownrc::{OwnRc, ReadRc, WriteRc},
        ownrcutils::read,
    },
};

use self::interpretation::{CellReaderTrait, CellTrait, CellWriterTrait, GroupTrait, LabelType};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_URL: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value"],
    target_interpretations: &["split"],
    constructor: Cell::from_cell,
};

#[derive(Debug)]
struct Data {
    text: String,
    pattern: String,
    spans: Vec<Span>,
}

#[derive(Debug, PartialEq)]
struct Span {
    text: String,
    delimiters_after: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    data: OwnRc<Data>,
    kind: Kind,
}

#[derive(Clone, Debug)]
enum Kind {
    Root,
    Span(usize),
}

#[derive(Debug)]
pub(crate) struct CellReader {
    data: ReadRc<Data>,
    kind: Kind,
}

#[derive(Debug)]
pub(crate) struct CellWriter {
    data: WriteRc<Data>,
    kind: Kind,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    data: OwnRc<Data>,
    kind: Kind,
}

implement_try_from_xell!(Cell, Split);

impl Cell {
    pub(crate) fn from_cell(origin: Xell, _: &str, params: &ElevateParams) -> Res<Xell> {
        let r = origin.read();
        let v = r.value()?;
        let text: String = v.as_cow_str().to_string();

        let Some(pattern_arg) = params.get(&Value::from(0)) else {
            return userres("regex requires a parameter");
        };

        let split_index = {
            if let Some(i) = params.get(&Value::from(1)) {
                if let OwnValue::Int(i) = i {
                    i.as_i128() as isize
                } else {
                    return userres("split index must be an integer");
                }
            } else {
                0
            }
        };

        let pattern_cow = pattern_arg.as_cow_str();
        let pattern = pattern_cow.as_ref();

        let spans = Self::do_split(&text, pattern, split_index)?;

        let root = Cell {
            data: OwnRc::new(Data {
                text: text.clone(),
                pattern: pattern.to_owned(),
                spans,
            }),
            kind: Kind::Root,
        };
        Ok(Xell::new_from(DynCell::from(root), Some(origin)))
    }

    fn do_split(text: &str, pattern: &str, max_splits: isize) -> Res<Vec<Span>> {
        #[derive(Debug)]
        enum SpanEnum {
            Text(String),
            Delimiters(usize),
        }

        let mut spans: Vec<SpanEnum> = vec![];
        // Iterate over each match of the pattern
        if max_splits >= 0 {
            let mut prev_index = 0;
            let mut split_count = 0;
            for (start, t) in text.match_indices(pattern) {
                if start == prev_index {
                    if let Some(SpanEnum::Delimiters(x)) = spans.last_mut() {
                        *x += 1;
                    } else {
                        spans.push(SpanEnum::Delimiters(1));
                    }
                } else {
                    let t = &text[prev_index..start];
                    spans.push(SpanEnum::Text(t.to_owned()));
                    spans.push(SpanEnum::Delimiters(1));
                }
                prev_index = start + t.len();
                split_count += 1;
                if split_count == max_splits {
                    break;
                }
            }

            if prev_index < text.len() {
                spans.push(SpanEnum::Text(text[prev_index..].to_owned()));
            }
        } else {
            let mut prev_index = text.len();
            let mut split_count = 0;
            for (start, t) in text.rmatch_indices(pattern) {
                if start + t.len() == prev_index {
                    if let Some(SpanEnum::Delimiters(x)) = spans.last_mut() {
                        *x += 1;
                    } else {
                        spans.push(SpanEnum::Delimiters(1));
                    }
                } else {
                    let t = &text[start + t.len()..prev_index];
                    spans.push(SpanEnum::Text(t.to_owned()));
                    spans.push(SpanEnum::Delimiters(1));
                }
                prev_index = start;
                split_count -= 1;
                if split_count == max_splits {
                    break;
                }
            }

            if prev_index > 0 {
                spans.push(SpanEnum::Text(text[..prev_index].to_owned()));
            }
            spans.reverse();
        }

        // println!("pattern: {:?}", pattern);
        // println!("max_splits: {:?}", max_splits);
        // println!("spans: {:?}", spans);

        let mut ret = vec![];
        for s in &spans {
            match s {
                SpanEnum::Text(t) => {
                    ret.push(Span {
                        text: t.clone(),
                        delimiters_after: 0,
                    });
                }
                SpanEnum::Delimiters(d) => {
                    if let Some(ret) = ret.last_mut() {
                        ret.delimiters_after = *d;
                    } else {
                        ret.push(Span {
                            text: "".to_owned(),
                            delimiters_after: *d,
                        });
                    }
                }
            }
        }

        Ok(ret)
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        match self.kind {
            Kind::Root => Ok("root"),
            Kind::Span(i) => Ok("span"),
        }
    }

    fn value(&self) -> Res<Value<'_>> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Span(i) => {
                let Some(s) = self.data.spans.get(i) else {
                    return nores();
                };
                Ok(Value::from(&s.text))
            }
        }
    }

    fn label(&self) -> Res<Value<'_>> {
        nores()
    }

    fn index(&self) -> Res<usize> {
        match self.kind {
            Kind::Root => Ok(0),
            Kind::Span(i) => Ok(i),
        }
    }

    fn serial(&self) -> Res<String> {
        match self.kind {
            Kind::Root => {
                let mut s = String::new();
                for span in &self.data.spans {
                    s.push_str(&span.text);
                    for _ in 0..span.delimiters_after {
                        s.push_str(&self.data.pattern);
                    }
                }
                Ok(s)
            }
            Kind::Span(i) => Ok(i.to_string()),
        }
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Span(i) => {
                let Some(s) = self.data.spans.get_mut(i) else {
                    return nores();
                };
                s.text = value.as_cow_str().to_string();
                Ok(())
            }
        }
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "split"
    }

    fn read(&self) -> Res<CellReader> {
        Ok(CellReader {
            data: self
                .data
                .read()
                .ok_or_else(|| lockerr("cannot read split spans"))?,
            kind: self.kind.clone(),
        })
    }

    fn write(&self) -> Res<CellWriter> {
        Ok(CellWriter {
            data: self
                .data
                .write()
                .ok_or_else(|| lockerr("cannot write split spans"))?,
            kind: self.kind.clone(),
        })
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Span(_) => Ok((
                Cell {
                    data: self.data.clone(),
                    kind: Kind::Root,
                },
                Relation::Sub,
            )),
        }
    }

    fn sub(&self) -> Res<Self::Group> {
        match self.kind {
            Kind::Root => Ok(Group {
                data: self.data.clone(),
                kind: Kind::Root,
            }),
            _ => nores(),
        }
    }

    fn attr(&self) -> Res<Self::Group> {
        nores()
    }
}

impl GroupTrait for Group {
    type Cell = Cell;

    type CellIterator = std::iter::Empty<Res<Self::Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        match self.kind {
            Kind::Root => Ok(read(&self.data)?.spans.len()),
            Kind::Span(i) => read(&self.data)?
                .spans
                .get(i)
                .map(|s| s.text.len())
                .ok_or_else(noerr),
        }
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        match self.kind {
            Kind::Root => Ok(Cell {
                data: self.data.clone(),
                kind: Kind::Span(index),
            }),
            _ => nores(),
        }
    }

    fn get_all(&self, label: Value<'_>) -> Res<Self::CellIterator> {
        nores()
    }
}

#[cfg(test)]
#[test]
fn test_split_do_span() {
    let text = "1, 2, 3";
    let pattern = ",";
    let spans = Cell::do_split(text, pattern, 0).unwrap();
    assert_eq!(
        spans,
        vec![
            Span {
                text: "1".to_owned(),
                delimiters_after: 1
            },
            Span {
                text: " 2".to_owned(),
                delimiters_after: 1
            },
            Span {
                text: " 3".to_owned(),
                delimiters_after: 0
            },
        ]
    );
}
