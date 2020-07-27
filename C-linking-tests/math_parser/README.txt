
  This program takes in a math expression, and prints the the parse tree
	generator by either a recursive decent parser or a table parser.
	It then evaluates the answer


HOW TO USE
		Build the program by using the "make" command in bash in the folder with all the files
		Then use the below command
		
        Command: ./expr [option]

Options (NO '-' before options, example: ./expr t):
         t = Run using Table Parser
         r = Run using Recursive Decsent Parser
         ? = Shows information

Calculator can be quit at anytime by typing 'EXIT' or 'exit'

When inputing an expression, only whole numbers can be inputed.
However, the expressions are evaluted using doubles, so all rational numbers can be expressed.

Furthermore, functions can be used within expression.
Functions are written as "function_name(param1,param2...)"
Example: sqrt(pow(3,2)+pow(4,2))*cos(arctan(4/3))
        This will return 3

FUNCTIONS (x represents an expression):
        sin(x) - x in radians
        cos(x) - x in radians
        tan(x) - x in radians
        cosd(x) - x in degrees
        sind(x) - x in degrees
        tand(x) - x in degrees
        arcsin(x) - returns radians
        arccos(x) - returns radians
        arctan(x) - returns radians
        arcsind(x) - returns degrees
        arccosd(x) - returns degrees
        arctand(x) - returns degrees
        sqrt(x) - returns the square root of x if x >= 0
        pow(x, r) - returns x^r unless x < 0 and 0<r<1
        pi(x) - returns pi*x
        vlength(x1,y1,x2,y2) - returns the distance between points (x1,y1) and (x2,y2)
		
GRAMMAR USED

 PRODUCTION FOR TABLE PARSER
 $ represents empty

<expression> -> <group><etail>
 
<etail> -> +<expression>
<etail> -> $
<etail> -> -<expression>
 
<group> -> <factor><gtail>

<gtail> -> *<group>
<gtail> -> $
<gtail> -> /<group>
 
<factor> -> <number>
<factor> -> -<factor>
<factor> -> (<expression>)
<factor> -> <function>
 
<number> -> <digit><ntail>
 
<ntail> -> <number>
<ntail> -> $

<digit> -> 0|1|2|3|4|5|6|7|8|9
 
<string> -> <char><stail>
 
<stail> -> <string>
<stail> -> $
 
<char> ->  a|b|c|d|...|x|y|z
 
<function> -> <string><paramlist>
 
<paramlist> -> (<expression><ptail>)
 
<ptail> -> ,<expression><ptail>
<ptail> -> $
