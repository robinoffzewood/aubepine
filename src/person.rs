//! A Person is a struct that represents an employee that must be on-call
//! It contains the name of the person, as a string, and an attribute that represents their membership: Employee or subcontractor

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Person {
    pub name: String,
    pub membership: Membership,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Membership {
    Employee,
    Subcontractor,
}

impl Person {
    pub fn new_employee(name: String) -> Self {
        Self {
            name,
            membership: Membership::Employee,
        }
    }
    pub fn new_subcontractor(name: String) -> Self {
        Self {
            name,
            membership: Membership::Subcontractor,
        }
    }
}
