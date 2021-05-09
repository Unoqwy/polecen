use convert_case::{Case, Casing};
use syn::Ident;

pub(crate) trait ConvertCase {
    fn to_case(&self, case: Case) -> Self;
}

impl ConvertCase for Ident {
    fn to_case(&self, case: Case) -> Self {
        Ident::new(&self.to_string().to_case(case), self.span())
    }
}
