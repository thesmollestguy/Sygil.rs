# Types

## Numerical types

There are four kinds of numerical value types that can be split into 2 categories

Category one is the integer-type values, being the `short`, `int`, and `long` types which each represent varying ranges of numbers. These are created when you type a standard numerical value.

Category two is the float-type, which only includes the `float` type. This is created when typing a decimal-represented number.

## Booleans

A boolean is represented by the ? symbol followed by 0 for false and 1 for true

`?0` = False

`?1` = True

## Strings

Strings are values creates by placing any UTF-8 characters within the bounds of two `"` characters.

EX. `"Good Evening!"` => "Good Evening" string value

# Variables

A variable is a way to store a value without needing to repeat the same value and while being able to perform operations on the value.

## Declaration

To declare a variable, you need 4 ingredients: the declaration symbol (`:`), the variable's name (any string of 3 or more alphanumeric characters plus hyphens), the separator token (`;`), and the variable's value (any value type)

## Usage

To use the value stored within a variable you must use the `$` retrieval character followed by the variable's name. 

# Functions

Functions are ways to reuse and call code in a program without typing it multiple times

## Declaration

Simple functions are declared using the function declaration symbol (`::`) followed by a function identifier (any string of 3 or more alphanumeric characters plus hyphens), the function parameter brackets (`<>`, left empty for now), and the function code bracket (`|`). This is followed by the code held within the function and terminated with a function code bracket (`|`).

EX. ```::func<> |
    :var;0
|```

A more complex function starts with the same declaration (`::name<`) but alongside this has input parameters. These input parameters are formatted the same way normal variables are declared but without the value.

EX. ```::func<:param> |
    :var;$param
|```

## Calling

To call a function, you use the function call symbol (`_`) followed by the function identifier, opening function paramater bracket (`<`), any required parameters, and the closing function paramater bracket (`>`). 

EX. `_func<10>`

## Built-in Functions

There are 2 built-in functions in Sygil. 

`print`: This function prints each of the parameter inputs in succession followed by a newline character.

`return`: This function, when used inside of a function declaration, defines a function return value. This function can only be given a maximum of 1 parameter. 

# Classes

Classes are namespaces that are declarable and callable on-demand, making for better compartmentalization of programs.

## Declaration

To declare a class you need the class declaration symbol (`:@`) and a class identifier name. 

### Declaring sub-classes, sub-variables, and sub-functions

After declaring a class you can declare classes, variables, and functions below it using their standard declaration rules alongside the class's identifier. 

EX. 

`@class:var;2`

`@class:@classer`

`@class::func<>( _return<> )`