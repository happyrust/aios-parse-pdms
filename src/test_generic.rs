use std::hash::Hash;

fn max_value<T: Ord>(x: T, y: T) -> T{
    if x > y {
        x
    }else{
        y
    }
}


enum BookFormat { Paperback, Hardback, Ebook }
struct Book {
    isbn: i32,
    format: BookFormat,
}
impl PartialEq for Book {
    fn eq(&self, other: &Self) -> bool {
        self.isbn == other.isbn
    }
}
impl Eq for Book {}

trait A{
    fn xxx(&self) -> String;
}
trait B: A{
    fn yyy(&self) -> String{
        self.xxx()
    }
}

struct THashMap<K: Eq + Hash, V>{
    key: K,
    value: V,
}
#[derive(PartialEq, Eq, Hash)]
struct TestKey{

}

use std::fmt::{Debug, Display};
use std::ops::Mul;

fn console_log<T: Display> (x: T) {
    println!("{}", x);
}

#[derive(Clone, Copy, Debug)]
struct TestClone{
    num: i32,
}

fn square<T: Mul<Output = T> + Copy> (x: T) -> T {

    return x * x;
}


#[test]
fn test_generic() {

    let a = TestClone{ num: 23 };
    let b = a;

    dbg!(a);

    dbg!(max_value(1, 2));

    let map = THashMap{
        key: TestKey{},
        value: "test".to_string()
    };

    console_log("123213");


    let a = TestA{
        num:2
    };

    let b = TestB{
        num: 3.0
    };

}




trait Game{

}

trait GameState: std::marker::Sized + Debug {
    type G: Game;
    type B;
    fn generate_children(&self, game: &Self::G) -> Vec<Self>;
    fn get_initial_state(game: &Self::G) -> Self;
}

struct TestGame{

}

impl Game for TestGame{

}

// impl GameState for TestGame{
//     type G = TestGame;
//     type B = ();
//
//     fn generate_children(&self, game: &Self::G) -> Vec<Self> {
//         todo!()
//     }
//
//     fn get_initial_state(game: &Self::G) -> Self {
//         todo!()
//     }
// }


//associate type
pub trait ATrait{
    type T: std::fmt::Display;
    fn get_data(&self) -> Self::T;
    fn print_data(&self){
        println!("{}", self.get_data());
    }
}


pub trait GTrait<T>{
    fn get_data(&self) -> T;
    // fn print_data(&self){
    //     println!("{:?}", self.get_data());
    // }
}

pub struct TestA{
    pub num: i32,
}

// impl GTrait for TestA{
//     fn get_data(&self) -> i32 {
//        self.num
//     }
// }

// impl ATrait for TestA{
//     type T = i32;
//
//     fn get_data(&self) -> i32 {
//         self.num
//     }
// }

pub struct TestB{
    pub num: f32,
}

// impl ATrait for TestB{
//     type T = f32;
//
//     fn get_data(&self) -> f32 {
//         self.num
//     }
// }



pub trait Iterator<T> {
    fn next(&mut self) -> Option<T>;
}