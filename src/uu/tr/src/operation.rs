use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until},
    character::complete::one_of,
    multi::many0,
    sequence::{separated_pair, tuple},
    IResult,
};
use std::{
    collections::HashMap,
    io::{BufRead, Write},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Sequence {
    Char(char),
    CharRange(Vec<char>),
}

impl Sequence {
    pub fn parse_set_string(input: &str) -> Vec<Sequence> {
        many0(alt((
            alt((
                Sequence::parse_octal,
                Sequence::parse_backslash,
                Sequence::parse_audible_bel,
                Sequence::parse_backspace,
                Sequence::parse_form_feed,
                Sequence::parse_newline,
                Sequence::parse_return,
                Sequence::parse_horizontal_tab,
                Sequence::parse_vertical_tab,
            )),
            alt((
                Sequence::parse_char_range,
                Sequence::parse_char_star,
                Sequence::parse_char_repeat,
            )),
            alt((
                Sequence::parse_alnum,
                Sequence::parse_alpha,
                Sequence::parse_blank,
                Sequence::parse_control,
                Sequence::parse_digit,
                Sequence::parse_graph,
                Sequence::parse_lower,
                Sequence::parse_print,
                Sequence::parse_punct,
                Sequence::parse_space,
                Sequence::parse_space,
                Sequence::parse_upper,
                Sequence::parse_xdigit,
                Sequence::parse_char_equal,
                Sequence::parse_char,
            )),
        )))(input)
        .map(|(_, r)| r)
        .unwrap()
    }

    pub fn dissolve(self) -> Vec<char> {
        match self {
            Sequence::Char(c) => vec![c],
            Sequence::CharRange(r) => r,
        }
    }

    /// Sequence parsers

    fn parse_char(input: &str) -> IResult<&str, Sequence> {
        take(1usize)(input).map(|(l, r)| (l, Sequence::Char(r.chars().next().unwrap())))
    }

    fn parse_octal(input: &str) -> IResult<&str, Sequence> {
        tuple((
            tag("\\"),
            one_of("01234567"),
            one_of("01234567"),
            one_of("01234567"),
        ))(input)
        .map(|(l, (_, a, b, c))| {
            (
                l,
                Sequence::Char(
                    // SAFETY: All the values from \000 to \777 is valid based on a test below...
                    std::char::from_u32(
                        a.to_digit(8).unwrap() * 8 * 8
                            + b.to_digit(8).unwrap() * 8
                            + c.to_digit(8).unwrap(),
                    )
                    .unwrap(),
                ),
            )
        })
    }

    fn parse_backslash(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("\\")))(input).map(|(l, _)| (l, Sequence::Char('\\')))
    }

    fn parse_audible_bel(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("a")))(input).map(|(l, _)| (l, Sequence::Char('\u{0007}')))
    }

    fn parse_backspace(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("b")))(input).map(|(l, _)| (l, Sequence::Char('\u{0008}')))
    }

    fn parse_form_feed(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("f")))(input).map(|(l, _)| (l, Sequence::Char('\u{000C}')))
    }

    fn parse_newline(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("n")))(input).map(|(l, _)| (l, Sequence::Char('\u{000A}')))
    }

    fn parse_return(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("r")))(input).map(|(l, _)| (l, Sequence::Char('\u{000D}')))
    }

    fn parse_horizontal_tab(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("t")))(input).map(|(l, _)| (l, Sequence::Char('\u{0009}')))
    }

    fn parse_vertical_tab(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("\\"), tag("v")))(input).map(|(l, _)| (l, Sequence::Char('\u{000B}')))
    }

    fn parse_char_range(input: &str) -> IResult<&str, Sequence> {
        separated_pair(take(1usize), tag("-"), take(1usize))(input).map(|(l, (a, b))| {
            (l, {
                let (start, end) = (
                    u32::from(a.chars().next().unwrap()),
                    u32::from(b.chars().next().unwrap()),
                );
                if (start >= 97 && start <= 122 && end >= 97 && end <= 122 && end > start)
                    || (start >= 65 && start <= 90 && end >= 65 && end <= 90 && end > start)
                    || (start >= 48 && start <= 57 && end >= 48 && end <= 57 && end > start)
                {
                    Sequence::CharRange(
                        (start..=end)
                            .map(|c| std::char::from_u32(c).unwrap())
                            .collect(),
                    )
                } else {
                    // This part is unchecked...not all `u32` => `char` is valid
                    Sequence::CharRange(
                        (start..=end)
                            .filter_map(|c| std::char::from_u32(c))
                            .collect(),
                    )
                }
            })
        })
    }

    fn parse_char_star(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("["), take(1usize), tag("*"), tag("]")))(input).map(|(_, (_, _, _, _))| todo!())
    }

    fn parse_char_repeat(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("["), take(1usize), tag("*"), take_until("]"), tag("]")))(input).map(
            |(l, (_, c, _, n, _))| {
                (
                    l,
                    Sequence::CharRange(
                        std::iter::repeat(c.chars().next().unwrap())
                            .take(n.parse().unwrap())
                            .collect(),
                    ),
                )
            },
        )
    }

    fn parse_alnum(input: &str) -> IResult<&str, Sequence> {
        tag("[:alnum:]")(input).map(|(l, _)| {
            (
                l,
                Sequence::CharRange(('a'..='z').chain('A'..'Z').chain('0'..'9').collect()),
            )
        })
    }

    fn parse_alpha(input: &str) -> IResult<&str, Sequence> {
        tag("[:alpha:]")(input).map(|(l, _)| {
            (
                l,
                Sequence::CharRange(('a'..='z').chain('A'..'Z').collect()),
            )
        })
    }

    fn parse_blank(input: &str) -> IResult<&str, Sequence> {
        tag("[:blank:]")(input).map(|(_, _)| todo!())
    }

    fn parse_control(input: &str) -> IResult<&str, Sequence> {
        tag("[:cntrl:]")(input).map(|(_, _)| todo!())
    }

    fn parse_digit(input: &str) -> IResult<&str, Sequence> {
        tag("[:digit:]")(input).map(|(l, _)| (l, Sequence::CharRange(('0'..='9').collect())))
    }

    fn parse_graph(input: &str) -> IResult<&str, Sequence> {
        tag("[:graph:]")(input).map(|(_, _)| todo!())
    }

    fn parse_lower(input: &str) -> IResult<&str, Sequence> {
        tag("[:lower:]")(input).map(|(_, _)| todo!())
    }

    fn parse_print(input: &str) -> IResult<&str, Sequence> {
        tag("[:print:]")(input).map(|(_, _)| todo!())
    }

    fn parse_punct(input: &str) -> IResult<&str, Sequence> {
        tag("[:punct:]")(input).map(|(_, _)| todo!())
    }

    fn parse_space(input: &str) -> IResult<&str, Sequence> {
        tag("[:space:]")(input).map(|(_, _)| todo!())
    }

    fn parse_upper(input: &str) -> IResult<&str, Sequence> {
        tag("[:upper:]")(input).map(|(l, _)| (l, Sequence::CharRange(('A'..='Z').collect())))
    }

    fn parse_xdigit(input: &str) -> IResult<&str, Sequence> {
        tag("[:xdigit:]")(input).map(|(_, _)| todo!())
    }

    fn parse_char_equal(input: &str) -> IResult<&str, Sequence> {
        tuple((tag("[="), take(1usize), tag("=]")))(input).map(|(_, (_, _, _))| todo!())
    }
}

pub trait SymbolTranslatorNew {
    fn translate(&mut self, current: char) -> Option<char>;
}

#[derive(Debug, Clone)]
pub struct DeleteOperationNew {
    set: Vec<Sequence>,
    complement_flag: bool,
}

impl DeleteOperationNew {
    pub fn new(set: Vec<Sequence>, complement_flag: bool) -> DeleteOperationNew {
        DeleteOperationNew {
            set,
            complement_flag,
        }
    }
}

impl SymbolTranslatorNew for DeleteOperationNew {
    fn translate(&mut self, current: char) -> Option<char> {
        let found = self.set.iter().any(|sequence| match sequence {
            Sequence::Char(c) => c.eq(&current),
            Sequence::CharRange(r) => r.iter().any(|c| c.eq(&current)),
        });
        (self.complement_flag == found).then(|| current)
    }
}

#[derive(Debug, Clone)]
pub enum TranslateOperationNew {
    Standard(HashMap<char, char>),
    Complement(u32, Vec<char>, Vec<char>, char, HashMap<char, char>),
}

impl TranslateOperationNew {
    fn next_complement_char(mut iter: u32) -> (u32, char) {
        while let None = char::from_u32(iter) {
            iter = iter.saturating_add(1)
        }
        (iter, char::from_u32(iter).unwrap())
    }
}

impl TranslateOperationNew {
    pub fn new(
        set1: Vec<Sequence>,
        mut set2: Vec<Sequence>,
        truncate_set2: bool,
        complement: bool,
    ) -> TranslateOperationNew {
        let fallback = set2.last().cloned().unwrap();
        println!("fallback:{:#?}", fallback);
        if truncate_set2 {
            set2.truncate(set1.len());
        }
        if complement {
            TranslateOperationNew::Complement(
                0,
                set1.into_iter().flat_map(Sequence::dissolve).collect(),
                set2.into_iter()
                    .flat_map(Sequence::dissolve)
                    .rev()
                    .collect(),
                // TODO: Check how `tr` actually handles this
                fallback.dissolve().first().cloned().unwrap(),
                HashMap::new(),
            )
        } else {
            TranslateOperationNew::Standard(
                set1.into_iter()
                    .flat_map(Sequence::dissolve)
                    .zip(
                        set2.into_iter()
                            .chain(std::iter::repeat(fallback))
                            .flat_map(Sequence::dissolve),
                    )
                    .collect::<HashMap<_, _>>(),
            )
        }
    }
}

impl SymbolTranslatorNew for TranslateOperationNew {
    fn translate(&mut self, current: char) -> Option<char> {
        match self {
            TranslateOperationNew::Standard(map) => Some(
                map.iter()
                    .find_map(|(l, r)| l.eq(&current).then(|| *r))
                    .unwrap_or(current),
            ),
            TranslateOperationNew::Complement(iter, set1, set2, fallback, mapped_characters) => {
                // First, try to see if current char is already mapped
                // If so, return the mapped char
                // Else, pop from set2
                // If we popped something, map the next complement character to this value
                // If set2 is empty, we just map the current char directly to fallback --- to avoid looping unnecessarily
                if let Some(c) = set1.iter().find(|c| c.eq(&&current)) {
                    Some(*c)
                } else {
                    while let None = mapped_characters.get(&current) {
                        if let Some(p) = set2.pop() {
                            let (next_index, next_value) =
                                TranslateOperationNew::next_complement_char(*iter);
                            *iter = next_index;
                            mapped_characters.insert(next_value, p);
                        } else {
                            mapped_characters.insert(current, *fallback);
                        }
                    }
                    Some(*mapped_characters.get(&current).unwrap())
                }
            }
        }
    }
}

pub fn translate_input_new<T>(input: &mut dyn BufRead, output: &mut dyn Write, mut translator: T)
where
    T: SymbolTranslatorNew,
{
    let mut buf = String::new();
    let mut output_buf = String::new();
    while let Ok(length) = input.read_line(&mut buf) {
        if length == 0 {
            break;
        } else {
            let filtered = buf.chars().filter_map(|c| translator.translate(c));
            output_buf.extend(filtered);
            output.write_all(output_buf.as_bytes()).unwrap();
        }
        buf.clear();
        output_buf.clear();
    }
}

#[test]
fn test_parse_char_range() {
    assert_eq!(Sequence::parse_set_string(""), vec![]);
    assert_eq!(
        Sequence::parse_set_string("a-z"),
        vec![Sequence::CharRange(vec![
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
        ])]
    );
    assert_eq!(
        Sequence::parse_set_string("a-zA-Z"),
        vec![
            Sequence::CharRange(vec![
                'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
                'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
            ]),
            Sequence::CharRange(vec![
                'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P',
                'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
            ])
        ]
    );
    assert_eq!(
        Sequence::parse_set_string(", ┬─┬"),
        vec![
            Sequence::Char(','),
            Sequence::Char(' '),
            Sequence::Char('┬'),
            Sequence::Char('─'),
            Sequence::Char('┬')
        ]
    );
}

#[test]
fn test_parse_octal() {
    for a in '0'..='7' {
        for b in '0'..='7' {
            for c in '0'..='7' {
                assert!(
                    Sequence::parse_set_string(format!("\\{}{}{}", a, b, c).as_str()).len() == 1
                );
            }
        }
    }
}
