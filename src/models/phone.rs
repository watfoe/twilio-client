use std::str::FromStr;

use crate::error::ParseError;
use phonenumber::country::Id;
use phonenumber::{Mode, PhoneNumber};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawPhone {
    pub number: String,
    pub country_code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Phone {
    phone_number: PhoneNumber,
}

impl Phone {
    pub fn parse(number: &str, country_iso: &str) -> Result<Phone, ParseError> {
        let country_iso = country_iso.to_uppercase();
        let country_id: Id = Id::from_str(country_iso.as_str()).map_err(|_| {
            ParseError(format!(
                "{country_iso} is not a valid or known phone country code"
            ))
        })?;

        let Ok(parsed_phone) = phonenumber::parse(Some(country_id), number) else {
            return Err(ParseError(format!(
                "error while parsing phone number {number}"
            )));
        };
        let phone_valid = phonenumber::is_valid(&parsed_phone);

        if phone_valid {
            return Ok(Phone {
                phone_number: parsed_phone,
            });
        }

        Err(ParseError(format!("{number} is not a valid phone number.")))
    }

    pub fn parse_with_no_country(number: &str) -> Result<Phone, ParseError> {
        let Ok(parsed_phone) = phonenumber::parse(None, number) else {
            return Err(ParseError(format!(
                "error while parsing phone number {number}."
            )));
        };
        let phone_valid = phonenumber::is_valid(&parsed_phone);

        if phone_valid {
            return Ok(Phone {
                phone_number: parsed_phone,
            });
        }

        Err(ParseError(format!("{number} is not a valid phone number.")))
    }

    pub fn e164_number(&self) -> String {
        self.phone_number.format().mode(Mode::E164).to_string()
    }

    pub fn country_iso(&self) -> String {
        self.phone_number
            .country()
            .id()
            .unwrap()
            .as_ref()
            .to_string()
    }
}

impl PartialEq<Phone> for Phone {
    fn eq(&self, other: &Phone) -> bool {
        self.e164_number() == other.e164_number()
    }
}

#[cfg(test)]
mod tests {
    use claim::assert_err;
    use fake::Fake;
    use quickcheck::{Arbitrary, Gen};

    use crate::models::Phone;

    #[derive(Debug, Clone)]
    struct ValidPhoneFixture {
        country_id: String,
        number: String,
    }

    impl Arbitrary for ValidPhoneFixture {
        fn arbitrary(_g: &mut Gen) -> ValidPhoneFixture {
            let (number, country_id) = generate_phone();

            Self { number, country_id }
        }
    }

    fn generate_phone() -> (String, String) {
        let num = (1..9).fake::<u64>();
        let number = format!("+2547{num}");

        (number, "KE".to_string())
    }

    #[test]
    fn empty_string_is_rejected() {
        let phone = "";
        assert_err!(Phone::parse(phone, "KE"));
    }

    #[test]
    fn number_with_invalid_chars_is_rejected() {
        assert_err!(Phone::parse("2547ji@89898", "KE"));
    }

    #[test]
    fn number_with_length_not_in_range_is_rejected() {
        let test_cases = vec!["25470234323", "254723435456523"];

        for number in test_cases {
            assert_err!(Phone::parse(number, "KE"));
        }
    }

    #[quickcheck_macros::quickcheck]
    fn a_valid_phone_is_parsed_successfully(valid_phone: ValidPhoneFixture) -> bool {
        Phone::parse(&valid_phone.number, &valid_phone.country_id).is_ok()
    }

    #[quickcheck_macros::quickcheck]
    fn a_valid_phone_with_nc_is_parsed_successfully(valid_phone: ValidPhoneFixture) -> bool {
        Phone::parse_with_no_country(&valid_phone.number).is_ok()
    }

    #[quickcheck_macros::quickcheck]
    fn a_valid_phone_with_nc_has_correct_country(valid_phone: ValidPhoneFixture) -> bool {
        let phone = Phone::parse_with_no_country(&valid_phone.number).unwrap();
        phone.country_iso() == "KE".to_string()
    }
}
