[![Build Status](https://travis-ci.com/dusk-network/rusk-contract.svg?branch=master)](https://travis-ci.com/dusk-network/rusk-contract)
[![Repository](https://dusk-network.github.io/rusk-contract/repo-badge.svg)](https://github.com/dusk-network/rusk-contract)
[![Documentation](https://dusk-network.github.io/Hades252/badge.svg)](https://dusk-network.github.io/rusk-contract/index.html)

A set of macros for easily writing Rusk Smart Contracts with multiple end points.

Using those macros, it is possible to have multiple methods for one contract, with different signatures and signature's size.

**_If a contract only perform one single operation, this crate is not necessary._**

## Example

This is by far the most common scenario, with methods that takes arguments and returns `i32`, with an empty entry point as last function:

```ignore
#![no_std]
use rusk_contract;

#[rusk_contract::method(opcode = 1)]
pub fn sum(a: i32, b: i32) -> i32 {
    a + b
}

#[rusk_contract::method(opcode = 2)]
pub fn bit_and(x: u8, y: u8) -> i32 {
    (x & y) as i32
}

#[rusk_contract::main]
pub fn call() {}
```

The function annotated as `main` needs to be specified as last macro call - the macro will warn you otherwise.

The main's name is not important, since it will always be transformed into `call`, the entry point expected by the VM.

The visibility also is forced to `pub` even if omitted, for the same reason.

Any code written inside the function specified as `main` will be executed _before_ calling any of the method invoked by the VM. That allows common code to be executed, and also traps the method execution if it's needed.

## Example

```ignore
#[rusk_contract::main]
pub fn call() {
    // gets the current opcode
    let code: u8 = dusk_abi::opcode::<u8>();

    // if `bit_and` method was requested...
    if code == 2 {
        // execute it manually...
        let result = bit_and(dusk_abi::argument());
        // ..and performs a `bit or` on the result before return it
        dusk_abi::ret::<i32>(result | 0xf0);
    }
}
```

By default the main function returns a `i32`, conventionally used as _status exit code_. Notice that the default ones, `0`, is generally used as _generic unsuccessful execution_.

It's also possible change the value type returned, with the following constraints:

- All the methods needs to return the same type
- The type implements `Default` trait
- The type implements `Sized` trait (its size is known at compile time)
- The type implements `dataview::Pod` trait

## Example

```ignore
#[rusk_contract::method(opcode = 1)]
pub fn beef() -> [u8; 2] {
    [0xbe, 0xef]
}

#[rusk_contract::method(opcode = 2)]
pub fn ab(b: u8) -> [u8; 2] {
    [0xab, b]
}

#[rusk_contract::main]
pub fn call() -> [u8; 2] {}
```

Notice that functions marked as method can also don't have arguments, such as `beef` above.
