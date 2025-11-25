use anyhow::bail;

/// Set of verses (zero based)
#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub struct Verses(usize);

impl Verses {
    #[cfg(test)]
    pub const ONE: Verses = Verses(1);
    #[cfg(test)]
    pub const ONE_THREE: Verses = Verses(5);
    pub const TWO: Verses = Verses(2);
    #[cfg(test)]
    pub const TWO_FOUR: Verses = Verses(10);
    #[cfg(test)]
    pub const THREE: Verses = Verses(4);
    #[cfg(test)]
    pub const FIVE: Verses = Verses(16);

    // pub fn is_single(&self) -> bool {
    //     let verse = self.0.trailing_zeros();
    //     self.0 == 1 << verse
    // }

    pub fn to_single(&self) -> Option<usize> {
        let verse = self.0.trailing_zeros();
        if self.0 == 1 << verse {
            Some(verse as usize)
        } else {
            None
        }
    }
}

impl std::str::FromStr for Verses {
    type Err = anyhow::Error;

    fn from_str(number: &str) -> Result<Self, Self::Err> {
        match number {
            "1" => Ok(Verses(0b1)),
            "1,2" => Ok(Verses(0b11)),
            "1,2,3" => Ok(Verses(0b111)),
            "1,2,3,4" => Ok(Verses(0b1111)),
            "1,2,3,4,5" => Ok(Verses(0b11111)),
            "1,3" => Ok(Verses(0b101)),
            "2" => Ok(Verses(0b10)),
            "2,3" => Ok(Verses(0b110)),
            "2,3,4" => Ok(Verses(0b1110)),
            "2,4" => Ok(Verses(0b1010)),
            "3" => Ok(Verses(0b100)),
            "4" => Ok(Verses(0b1000)),
            "5" => Ok(Verses(0b10000)),
            "6" => Ok(Verses(0b100000)),
            number =>
                bail!("<ending number=`{number}`>"),
        }
    }
}

impl std::iter::Iterator for Verses {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            let next = self.0.trailing_zeros();
            self.0 -= 1 << next;
            Some(next as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_collect(s: &str, vs: &[usize]) {
        let verses: Verses = s.parse().expect("valid verses");
        let verses: Vec<usize> = verses.map(|v| v + 1).collect();
        assert_eq!(&verses, vs);
    }

    #[test]
    fn test_parse_and_collect() {
        parse_and_collect("1", &[1]);
        parse_and_collect("1,2", &[1, 2]);
        parse_and_collect("1,2,3", &[1, 2, 3]);
        parse_and_collect("1,3", &[1, 3]);
        parse_and_collect("2", &[2]);
        parse_and_collect("2,3", &[2, 3]);
        parse_and_collect("2,3,4", &[2, 3, 4]);
        parse_and_collect("2,4", &[2, 4]);
        parse_and_collect("3", &[3]);
        parse_and_collect("4", &[4]);
        parse_and_collect("5", &[5]);
    }
}