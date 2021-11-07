" " = \s+   -> ""       // optional space
__ = \s+    -> " "      // mandatory space
_ = \s*     -> " "      // recommended space


id = \a ( \a | \d )+
<=>
id.value

// 🔴 general blocks
// ( [ { < > } ] )
// "*?" below means non-greedy matching
blocks = round$r | squar$s | curly$c | angle$a | quote$q | other$o
    round = "(" blocks ")"
    squar = "[" blocks "]"
    curly = "{" blocks "}"
    angle = "<" blocks ">"
    quote = "\"" \.*? "\""
    other = \.*?
<=>
$r || $s || $c || $a || $q || $o



// 🔴 use
// use a::b;
// use a::b::c::*;
// use a::{b, c};
// use a::{b::{d,e::{f::*}}, c};
use = "use" __ list ";"
    list = path ( "," path )*
    path = ( id$begin ( "::" id$mid )* "::" )? term$end
    term = id | "*" | "{" list "}"
<=>
use:^keyword
    $begin^id
        nested($mid^id)
            $end

// 🔴 fn
// pub fn cell_str(s: &str) -> Cell {
//     Cell::from(s)
// }
fn = accessmod __ "fn" __ id$name . "(" . arguments? . ")" _ returnstmt . codeblock
    accessmod = "pub"?$mod
    arguments = ( id$argname . ":" _ type$argtype "," )+$args
    returnstmt = "->" _ type$rettype
    codeblock = "{" statement "}"
    statement =
<=>
fn:^fn
    $name^fn
        @mod:$mod^modifier
        arguments:
            $argname:  $argtype


