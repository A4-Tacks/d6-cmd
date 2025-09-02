A simple command vm

Supports value:

- int 64-bit
- int stack, only operation top int

Supports cmd:

- <*num*> : set next cmd arg, times arg when setted arg
- `%` <*var*> : using <*var*> value set next cmd arg
- `+` <*var*> : var += arg
- `-` <*var*> : var -= arg
- `=` <*var*> : assign var value to arg
- `$` <*var*> : resize stack (size += arg), convert to stack when <*var*> is int
- `{` <*var*> <*cmds*> `}` : define macro, name by <*var*>
- `@` <*var*> : call macro
- `[` <*cmds*> `]`: grouped cmds, `2[+a+b]` like `+a+b+a+b`
- `*` <*var*> : define a mark
- `^` <*var*> : jump to mark

Comment: `;` ...

Special vars:

- `?` : random integer [0,255]


Examples
===============================================================================

Fib calc

```text
$g ; new stack `g`
; f:fib(p) -> r [c d]
{f
    -p ; p--
    1=c%p[=c]%c[ ; if p>0
        1=r
    ]1=d%c[=d]%d[ ; else
        1$g%p=g ; g.push(p)
        @f
        %g=p ; p = g.top
        %r=g ; g.top = r
        -p@f
        %g+r ; r += g.top
        =d-d%d$g ; g.pop()
    ]
}
$h ; new stack `h`
=i
15[
    %i=p ; p=i
    @f ; r=fib(i)
    1$h%r=h ; h.push(r)
    +i ; i++
]
```

**Input this program to** `cargo run -` outputs:

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
     Running `target/debug/d6-cmd -`
g: []
h: [1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610]
i: 15
p: -1
c: 1
r: 610
d: -1

```
