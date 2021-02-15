// [ref](https://github.com/rust-lang-nursery/lazy-static.rs)
#[path="core_lazy.rs"]
pub mod lazy;

pub use core::ops::Deref as __Deref;

#[macro_export(local_inner_macros)]
macro_rules! __lazy_static_internal {
    // static refの中でわちゃわちゃやるタイプ
    ($(#[$attr:meta])* ($($vis:tt)*) static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        __lazy_static_internal!(@MAKE TY, $(#[$attr])*, ($($vis)*), $N);
        __lazy_static_internal!(@TAIL, $N : $T = $e);
        lazy_static!($($t)*);
    };
    // 一番シンプルなやつ？@TAILだから末尾なのかな？
    (@TAIL, $N:ident : $T:ty = $e:expr) => {
        impl $crate::__Deref for $N {
            type Target = $T;
            fn deref(&self) -> &$T {
                #[inline(always)]
                fn __static_ref_initialize() -> $T { $e }

                #[inline(always)]
                fn __stability() -> &'static $T {
                    __lazy_static_create!(LAZY, $T);
                    LAZY.get(__static_ref_initialize)
                }
                __stability()
            }
        }
        impl $crate::LazyStatic for $N {
            fn initialize(lazy: &Self) {
                let _ = &**lazy;
            }
        }
    };
    // structの場合
    (@MAKE TY, $(#[$attr:meta])*, ($($vis:tt)*), $N:ident) => {
        #[allow(missing_copy_implementations)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        $(#[$attr])*
        $($vis)* struct $N {__private_field: ()}
        $($vis)* static $N: $N = $N {__private_field: ()};
    };
    () => ()
}

#[macro_export(local_inner_macros)]
macro_rules! lazy_static {
//  使い方)
//    lazy_static! {
//        [pub] static ref NAME_1: TYPE_1 = EXPR_1;
//        [pub] static ref NAME_2: TYPE_2 = EXPR_2;
//        ...
//        [pub] static ref NAME_N: TYPE_N = EXPR_N;
//    }

//    例1)
//    #[macro_use]
//    extern crate lazy_static;
//    fn main() {
//        lazy_static! {
//            /// This is an example for using doc comment attributes
//            static ref EXAMPLE: u8 = 42;
//        }
//    }
//  例2)
//        #[macro_use]
//        extern crate lazy_static;
//        use std::collections::HashMap;
//        lazy_static! {
//            static ref HASHMAP: HashMap<u32, &'static str> = {
//                let mut m = HashMap::new();
//                m.insert(0, "foo");
//                m.insert(1, "bar");
//                m.insert(2, "baz");
//                m
//            };
//            static ref COUNT: usize = HASHMAP.len();
//            static ref NUMBER: u32 = times_two(21);
//        }
    ($(#[$attr:meta])* static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        __lazy_static_internal!($(#[$attr])* () static ref $N : $T = $e; $($t)*);
    };
    ($(#[$attr:meta])* pub static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        __lazy_static_internal!($(#[$attr])* (pub) static ref $N : $T = $e; $($t)*);
    };
    ($(#[$attr:meta])* pub ($($vis:tt)+) static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        __lazy_static_internal!($(#[$attr])* (pub ($($vis)+)) static ref $N : $T = $e; $($t)*);
    };
    () => ()
}

pub trait LazyStatic {
    fn initialize(lazy: &Self);
}

pub fn initialize<T: LazyStatic>(lazy: &T) {
    LazyStatic::initialize(lazy);
}
