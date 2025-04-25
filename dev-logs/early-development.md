<div align="center">

<h1>Early Development Log - Zekken Programming Language</h1>
<h3>This file contains updates from the initial development phase (February 2025 - April 2025).</h3>

</div>

### Early Development Build #1 (2/23/25):
- Initial github commit (a little bit into development)
- Added Lexer
- Added Parser

### Early Development Build #2 (2/25/25):
- Added `type` field to `VarDecl` struct in AST
- Improved Lexer with new operators

### Early Development Build #3 (2/26/25):
- Improved Function declaration
- Improved Lexer with data type support
- Updated String and Comment parsing

### Early Development Build #4 (2/27/25):
- Improved AST and Lexer
- Updated and improved Parser logic

### Early Development Build #5 (3/1/25):
- Added evaluators for code
- Improved environment handling
- Updated Lexer with new token type

### Early Development Build #6 (3/2/25):
- Improved evaluation of code
- Improved error messages
- Improved value formatting
- Added support for including and exporting values from different files

### Early Development Build #7 (3/3/25):
- Added error handling module for better errors/error messages
- Updated Lexer with new token type
- Improved `println` native function

### Early Development Build #8 (3/4/25):
- Updated Lexer for new data types
- Improved `println` native function further

### Early Development Build #9 (3/5/25):
- Implemented native library support
- Fixed type error for returning values

### Early Development Build #10 (3/7/25):
- Improved native library loading
- Improved error handling in general

### Early Development Build #11 (4/14/25):
- Improved executable config to make the size smaller
- Improved Object evaluation at runtime
- Added tests for the Fibonacci Sequence and BMI calculator

### Early Development Build #12 (4/15/25):
- Implemented user input from console
- Created tests for including exported values from files and user input
- Improved "math" library

### Early Development Build #13 (4/16/25):
- Added Array and Object indexing
- Fixed Object iteration order

### Early Development Build #14 (4/17/25):
- Fixed string concatenation
- Fixed Try-Catch statements not working as expected (specifically the catch block)
- Fixed While loops not working as expected

### Early Development Build #15 (4/18/25):
- Fixed variable reassignment not working 
- Fixed functions not being able to take in objects or arrays as parameter data types
- Made unexpected tokens log in the syntax error format
- Fixed boolean values not working for variable declarations
- Made escape characters in strings work properly
- Added `updates.md` file

### Early Development Build #16 (4/19/25):
- Fixed how comments are parsed
- Updated `README.md`
- Moved `tests` directory out of `src` directory
- Fixed an issue where commenting out a native function would still cause the function to execute

### Early Development Build #17 (4/21/25):
- Removed comments from AST
- Improved Parser
- Updated The logo image
- Improved optimization/disk size of the executable when built

### Early Development Build #18 (4/24/25):
- Added support for lambda functions
- Moved `fibonacci.zk` and `bmi_calc.zk` from the `tests` directory to a new `examples` directory
- Fixed a parser bug (the bug wasn't a major one but it bugged me, get it?)
- Created a `dev-log` directory that holds all development logs, for each version, such as this version