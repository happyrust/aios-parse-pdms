use nom::multi::many_till;
use nom::bytes::complete::{tag, take_till, take_until};
use nom::combinator::{map, not, recognize, value, verify};
use nom::IResult;
use nom::number::complete::be_i32;
use nom::sequence::preceded;
use crate::test_cases::convert_str_to_bytes;


fn parser(s: &[u8]) -> IResult<&[u8], (Vec<i32>, &[u8])> {
    many_till(map(be_i32, |x| x), tag([0x0, 0x0, 0x0, 0x7]))(s)
}

// fn last_steno_entry(input: &str) -> IResult<&str, (String, String, Option<String>)> {
//     let (input, (steno_group, (contents, _))) = tuple((
//         steno_group,
//         many_till(
//             alt((cxcomment, non_comment)),
//             tag(r"}"))))(input)?;
//     let translation = contents.iter()
//         .map(|obj| match obj { TranslationItem::NotComment(s) => s.as_str(), _ => "" })
//         .collect::<Vec<&str>>().join("").trim().to_string();
//     let comment = match contents.iter()
//         .map(|obj| match obj { TranslationItem::Comment(s) => s.as_str(), _ => "" })
//         .collect::<Vec<&str>>().join("").trim() { "" => None, s => Some(s.to_string()) };
//     Ok((input, (steno_group, translation, comment)))
// }

#[test]
pub fn test_take_till(){
    let test_data = "\
    00 00 00 00 00 00 00 07 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 01";
    let data = convert_str_to_bytes(test_data);
    // let s = parser(data.as_slice());
    let s: IResult<&[u8], (Vec<i32>, i32)> = many_till(verify(be_i32, |x| *x == 0), verify(be_i32, |x| *x == 7))(data.as_slice());
    // let s: IResult<&[u8], (Vec<bool>, &[u8])>  = pmany_till(map(be_i32, |x| x == 0), tag([0x0, 0x0, 0x0, 0x7]))(data.as_slice());
    // let s: IResult<&[u8], &[u8]> = take_until( map(be_i32, |x| x != 0))(data.as_slice());
    dbg!(s);
    // many_till( tag( "ab", () ), tag("ef", ()))("ababefg");

}