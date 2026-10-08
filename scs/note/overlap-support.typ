// Typst port of overlap-support.tex
// Compile: typst compile overlap-support.typ overlap-support-typst.pdf

// ---------------------------------------------------------------------------
// Page, fonts, paragraphs
// ---------------------------------------------------------------------------
#let linkcolor = rgb(0, 0, 115) // blue!45!black

#set document(
  title: [The forced-count graph lives on prefixes and suffixes],
  date: datetime(year: 2026, month: 10, day: 8),
)
#set page(paper: "us-letter", margin: 1in, numbering: "1")
#set text(font: "New Computer Modern", size: 11pt, lang: "en")
#show math.equation: set text(font: "New Computer Modern Math")
#show raw: set text(font: "Latin Modern Mono", size: 1.25em)
#set par(justify: true, leading: 0.55em, spacing: 0.55em, first-line-indent: 17pt)
#show link: set text(fill: linkcolor)

// Manual paragraph indent (for paragraphs that follow a theorem-like block,
// which LaTeX indents but Typst does not).
#let ind = h(17pt)

// ---------------------------------------------------------------------------
// Headings
// ---------------------------------------------------------------------------
#set heading(numbering: "1")
#show heading.where(level: 1): it => {
  counter(figure.where(kind: "thm")).update(0)
  counter(math.equation).update(0)
  block(above: 22pt, below: 13pt, sticky: true, text(size: 14.4pt, weight: "bold", {
    if it.numbering != none { counter(heading).display(it.numbering); h(1em) }
    it.body
  }))
}
// \paragraph{...}: run-in bold heading
#let paragraph(title) = { parbreak(); v(8pt); strong(title); h(1em) }

// ---------------------------------------------------------------------------
// Equations: numbered within section, only when explicitly requested
// ---------------------------------------------------------------------------
#let eqnum = (..n) => "(" + str(counter(heading).get().first()) + "." + str(n.pos().first()) + ")"
#set math.equation(numbering: none)
#show math.equation.where(block: true): set block(above: 11pt, below: 11pt)
#let numeq(eq) = math.equation(block: true, numbering: eqnum, eq.body)

// ---------------------------------------------------------------------------
// Theorem-like environments: one shared counter, numbered within section
// ---------------------------------------------------------------------------
#show figure.where(kind: "thm"): set block(breakable: true)
#show figure.where(kind: "thm"): it => block(width: 100%, above: 1.1em, below: 1.1em, {
  set align(left)
  set par(first-line-indent: (amount: 17pt, all: false))
  context {
    let num = str(counter(heading).get().first()) + "." + str(it.counter.get().first())
    strong[#it.supplement #num]
    if it.caption != none [ (#it.caption.body)]
    strong[.]
  }
  it.body
})

#let thm-env(supplement, italic: true) = (name: none, body) => figure(
  kind: "thm",
  supplement: supplement,
  numbering: "1",
  outlined: false,
  caption: name,
  {
    set text(style: if italic { "italic" } else { "normal" })
    body
  },
)
#let theorem = thm-env("Theorem")
#let lemma = thm-env("Lemma")
#let corollary = thm-env("Corollary")
#let conjecture = thm-env("Conjecture")
#let definition = thm-env("Definition", italic: false)
#let remark = thm-env("Remark", italic: false)
#let example = thm-env("Example", italic: false)

#let qedbox = box(width: 0.65em, height: 0.65em, stroke: 0.4pt, baseline: 0pt)
#let proof(body) = block(width: 100%, above: 0.9em, below: 1.1em, {
  set par(first-line-indent: (amount: 17pt, all: false))
  [_Proof._]
  body
  h(0.5em)
  h(1fr)
  qedbox
})

// ---------------------------------------------------------------------------
// References (cleveref-style: "Lemma 2.1", equations as "(2.1)")
// ---------------------------------------------------------------------------
#show ref: it => {
  let el = it.element
  if el == none { return it }
  let loc = el.location()
  let sec = counter(heading).at(loc).first()
  if el.func() == math.equation {
    let n = counter(math.equation).at(loc).first()
    link(loc, "(" + str(sec) + "." + str(n) + ")")
  } else if el.func() == figure and el.kind == "thm" {
    let n = counter(figure.where(kind: "thm")).at(loc).first()
    link(loc, [#el.supplement #sec.#n])
  } else {
    text(fill: linkcolor, it)
  }
}
// \Cref{a,b} for two labels of the same kind: "Theorems 4.1 and 4.2"
#let cref2(plural, a, b) = context {
  let num(l) = {
    let loc = locate(l)
    str(counter(heading).at(loc).first()) + "." + str(counter(figure.where(kind: "thm")).at(loc).first())
  }
  link(locate(a), plural + " " + num(a))
  [ and ]
  link(locate(b), num(b))
}

// ---------------------------------------------------------------------------
// Math macros
// ---------------------------------------------------------------------------
#let pref = math.op("pref")
#let suf = math.op("suf")
#let Pre = $cal(P)$
#let Suf = $cal(Q)$
#let Ov = $cal(O)$
#let cS = $cal(S)$
#let lsh(x) = $#x^(arrow.l)$
#let bigp(x) = math.lr($(#x)$, size: 1.2em)
#let tt(s) = math.mono(s)

// ---------------------------------------------------------------------------
// Title and abstract
// ---------------------------------------------------------------------------
#v(41pt)
#align(center)[
  #text(size: 17.28pt)[The forced-count graph lives on prefixes and suffixes]
  #v(1pt)
  #text(size: 12pt)[Notes toward a practical version of the polynomial-time SCS 2-approximation]
  #v(42pt)
  #text(size: 12pt)[October 8, 2026]
]
#v(26pt)

#block(inset: (x: 2.5em))[
  #set text(size: 10pt)
  #align(center, strong[Abstract])
  #v(5pt)
  #h(15pt)We study the count construction of Section~2 of _A Polynomial-Time 2-Approximation for Shortest Common Superstring_ (OpenAI, September 2026), which is defined over all $O(L^2)$ distinct substrings of the input. We prove that its base graph is supported on the at most $2L$ prefixes and suffixes of the input strings, and that the periodicity rule may be restricted, without changing any count, to pairs of words that are each simultaneously a prefix and a suffix of input strings. Both statements follow from a single _shift lemma_. Finally, we give a closed form for the blocking sums in terms of the edge multiplicities, which yields an algorithm that never materializes the substring set. We also show that restricting the rule to least periods is _not_ sound for this purpose. Finally, we implement the full connection phase of the preprint on top of these results. In experiments against an efficient greedy implementation, a simple order-merge post-pass makes the output competitive: it is shorter than greedy on repeat-rich inputs and on greedy's classical bad family, and it is often certified optimal by the lower bound $W$.
]

// ---------------------------------------------------------------------------
= Setting

We use the notation of the preprint without change: the reduced instance $cS$ (substring-free, no empty or duplicate strings), the set $V$ of its distinct substrings, the blocking functional $B_A (s;m)$ of (2.2), the counts $m$ defined by the extension rules (2.4), the requirement rule, and the period rule (2.5), and the base multiplicities $u(s) = m(s) - sum_c m(c s)$, $d(s) = m(s) - sum_c m(s c)$. Missing words have count zero. Write
$
  lambda(s) = sum_(c in Sigma) m(c s), #h(2em) rho(s) = sum_(c in Sigma) m(s c),
$
so that $u(s) = m(s) - lambda(s)$ and $d(s) = m(s) - rho(s)$. Let $Pre$ and $Suf$ be the sets of nonempty prefixes and nonempty suffixes of the strings in $cS$, and let $Ov = Pre inter Suf$ be the set of _overlap words_. Clearly $abs(Pre), abs(Suf) <= L$, and every required string lies in $Ov$.

#definition[
  A _rule_ is a quadruple $(A, v, w, k)$ as in (2.5): $A$ is a periodic text of period $p$, $v = A\[0:abs(v)\) in V$, $w$ is a nonempty prefix of $v$, and $abs(v) = abs(w) + k p$ with $k >= 1$. It is _triggered_ if $m(v) > B_A (v;m)$; it then imposes $m(w) >= B_A (w;m) + k + 1$, its _bound_. By construction, $m(s)$ is the maximum of $lambda(s)$, $rho(s)$, the requirement indicator, and the bounds of all triggered rules with $w = s$.
]

#ind For a text $A$ write $lsh(A)$ for its translate $lsh(A) (i) = A(i - 1)$. If $A\[0:abs(s)\) = s$ and $a = A(-1)$, then $lsh(A) \[0:abs(s) + 1\) = a s$, so $B_(lsh(A)) (a s;m)$ is defined. Translating a period-$p$ text yields a period-$p$ text.

Two facts about $v$ and $w$ in a rule are used repeatedly. Since $w$ is a prefix of $v$, $w in.not Pre$ implies $v in.not Pre$. Since $v$ has period $p$ and $abs(v) - abs(w) = k p$, also $w = v\[k p:abs(v)\)$, so $w$ is a suffix of $v$, and $w in.not Suf$ implies $v in.not Suf$.

// ---------------------------------------------------------------------------
= Two elementary lemmas

#lemma(name: [Prefix-free sums])[
  Let $y$ be a nonempty word and $Z$ a finite prefix-free set of nonempty words. Then $sum_(z in Z) m(y z) <= m(y)$. Symmetrically, if $Z$ is suffix-free, then $sum_(z in Z) m(z y) <= m(y)$.
] <lem:pfree>

#proof[
  Induct on the maximum length in $Z$. Group $Z$ by first letter $c$. If the one-letter word $c$ lies in $Z$, prefix-freeness makes it the only member beginning with $c$. Otherwise $Z_c = {z' : c z' in Z}$ is prefix-free, and induction gives $sum_(z' in Z_c) m(y c z') <= m(y c)$. Hence $sum_(z in Z) m(y z) <= sum_c m(y c) <= m(y)$ by the right extension rule. (If $y in.not V$, every term is zero.) The suffix-free case is the mirror image, using the left extension rule.
]

#lemma(name: [Bracket decomposition])[
  Let $A$ be periodic with $A\[0:abs(s)\) = s$, and put $a = A(-1)$, $e = A(abs(s))$. Then
  #context {
    let l1 = $Lambda_A (s) + B_(lsh(A)) (a s;m),$
    let l2 = $R_A (s) + B_A (s e;m),$
    let r1 = $Lambda_A (s)$
    let r2 = $R_A (s)$
    let s1 = $display(sum_(c != a)) m(c s),$
    let s2 = $display(sum_(d != e)) m(s d),$
    let wd(x) = measure(x).width
    let w1 = calc.max(wd(l1), wd(l2))
    let w2 = calc.max(wd(r1), wd(r2))
    let w3 = calc.max(wd(s1), wd(s2))
    set par(first-line-indent: 0pt)
    [#numeq($B_A (s;m) = Lambda_A (s) + B_(lsh(A)) (a s;m), #h(w1 - wd(l1)) #h(2.5em) #h(w2 - wd(r1)) Lambda_A (s) <= sum_(c != a) m(c s), #h(w3 - wd(s1))$) <eq:left-decomp>]
    [#numeq($B_A (s;m) = R_A (s) + B_A (s e;m), #h(w1 - wd(l2)) #h(2.5em) #h(w2 - wd(r2)) R_A (s) <= sum_(d != e) m(s d), #h(w3 - wd(s2))$) <eq:right-decomp>]
  }
  where $Lambda_A (s)$ counts, with multiplicity $m(r)$, the bracketing placements of $s$ whose left mismatch is immediately before $s$, and $R_A (s)$ those whose right mismatch is immediately after $s$.
] <lem:decomp>

#proof[
  Consider a counted placement $j$ of $s$ in $r$ as in (2.1)--(2.2).

  _Left._ If $j >= 2$, then position $j - 1$ is interior, so $r(j - 1) = A(-1) = a$ and $a s$ occurs at $j' = j - 1 >= 1$. Substituting $A(i) = lsh(A) (i + 1)$ turns each condition of (2.1) for $(A, s, j)$ into the corresponding condition for $(lsh(A), a s, j')$: the length condition $j' + abs(a s) = j + abs(s) <= abs(r) - 1$, the interior agreement $r(h) = A(h - j) = lsh(A) (h - j')$, and the two end mismatches $r(0) != A(-j) = lsh(A) (-j')$ and $r(abs(r) - 1) != A(abs(r) - 1 - j) = lsh(A) (abs(r) - 1 - j')$. The converse substitution is equally direct, so these placements are in bijection with the counted placements of $a s$ relative to $lsh(A)$, giving the second term. If $j = 1$, then $r = c thin s thin x thin d$ with $c != a$, $x = A\[abs(s):abs(s) + abs(x)\)$ and $d != A(abs(s) + abs(x))$; the placement is unique for this $r$. For fixed $c$ the tails $x d$ form a prefix-free set, since two of them first differ where one leaves $A$. @lem:pfree bounds their total by $m(c s)$.

  _Right._ Mirror image: a placement whose right mismatch is not adjacent to $s$ has $r(j + abs(s)) = A(abs(s)) = e$ and is exactly a counted placement of $s e$ relative to $A$. The adjacent ones have the form $r = y thin s thin d$ with $d != e$, where the heads $y = c thin A\[-abs(y) + 1:0\)$, $c != A(-abs(y))$, form a suffix-free set.
]

#corollary(name: [Monotonicity of the unblocked surplus])[
  Put $U_A (s) = m(s) - B_A (s;m)$. Then $U_A (s) >= 0$, $U_A (s) >= U_A (s e)$ and $U_A (s) >= U_(lsh(A)) (a s)$. In particular, for fixed $A$ and $w$, the rules $(A, A\[0:abs(w) + k p\), w, k)$ are triggered exactly for $k$ up to some threshold, and only the largest triggered $k$ gives a binding bound.
] <cor:mono>

#proof[
  By @eq:right-decomp and the extension rule, $m(s) >= m(s e) + sum_(d != e) m(s d) >= m(s e) + R_A (s)$; subtract $B_A (s;m) = R_A (s) + B_A (s e;m)$. The left inequality is the mirror image. Nonnegativity follows by induction on decreasing length, since $U_A (s e) = 0$ when $s e in.not V$. If $A\[0:abs(w) + k p\) in V$, so are its shorter prefixes, and the bound $B_A (w;m) + k + 1$ increases with $k$.
]

// ---------------------------------------------------------------------------
= The shift lemma

#lemma(name: [Shift lemma])[
  Let $(A, v, w, k)$ be a triggered rule.
  #set enum(numbering: n => text(style: "normal", numbering("(a)", n)), indent: 7pt, body-indent: 5.5pt, spacing: 13pt)
  #show enum: set block(above: 10pt, below: 10pt)
  + If $m(v) = lambda(v)$, then $lambda(w) >= B_A (w;m) + k + 1$.
  + If $m(v) = rho(v)$, then $rho(w) >= B_A (w;m) + k + 1$.
] <lem:shift>

#proof[
  (a) Let $a = A(-1)$; it is the left neighbor in $A$ of both $v$ and $w$. By @eq:left-decomp for $v$ and the trigger,
  $
    m(a v) + sum_(c != a) m(c v) = lambda(v) = m(v) > B_A (v;m)
    = Lambda_A (v) + B_(lsh(A)) (a v;m) <= sum_(c != a) m(c v) + B_(lsh(A)) (a v;m).
  $
  Hence $m(a v) > B_(lsh(A)) (a v;m) >= 0$, so $a v in V$. Now $(lsh(A), a v, a w, k)$ is a rule: $lsh(A)$ has period $p$, $a v = lsh(A) \[0:abs(v) + 1\) in V$, $a w$ is a nonempty prefix of $a v$, and $abs(a v) - abs(a w) = k p$. It is triggered, so the definition of $m$ at the longer word $a w$ gives $m(a w) >= B_(lsh(A)) (a w;m) + k + 1$. Applying @eq:left-decomp to $w$,
  $
    lambda(w) = m(a w) + sum_(c != a) m(c w) >= B_(lsh(A)) (a w;m) + k + 1 + Lambda_A (w)
    = B_A (w;m) + k + 1.
  $

  #ind (b) Let $e = A(abs(v))$. Since $abs(v) - abs(w) = k p$, also $e = A(abs(w))$. By @eq:right-decomp for $v$ and the trigger, exactly as in (a), $m(v e) > B_A (v e;m)$, so $v e in V$. Then $(A, v e, w e, k)$ is a triggered rule: $v e = A\[0:abs(v) + 1\)$, its prefix $A\[0:abs(w) + 1\) = w e$, and $abs(v e) - abs(w e) = k p$. Hence $m(w e) >= B_A (w e;m) + k + 1$, and @eq:right-decomp for $w$ gives $rho(w) = m(w e) + sum_(d != e) m(w d) >= B_A (w e;m) + k + 1 + R_A (w) = B_A (w;m) + k + 1$.
]

#remark[
  In words: if every counted occurrence of $v$ extends to the left, then the $k + 1$ unblocked copies of $w$ that a rule extracts from $v$ can be shifted one letter to the left, as copies of $a w$ in $a v$. The rule is then already implied by the left extension inequality at $w$. Part (b) uses the extension to the right, together with the period: $w$ and $v$ have the same right neighbor $e$ in $A$.
]

// ---------------------------------------------------------------------------
= Main results

#theorem(name: [Prefix/suffix support])[
  For every nonempty $s in V$:
  $
    s in.not Pre #h(0.5556em) ==> #h(0.5556em) m(s) = lambda(s), #h(2em)
    s in.not Suf #h(0.5556em) ==> #h(0.5556em) m(s) = rho(s).
  $
  Consequently $u(s) > 0$ implies $s in Pre$, and $d(s) > 0$ implies $s in Suf$. Every up edge of the base graph $G$ joins two words of $Pre union {epsilon}$, every down edge joins two words of $Suf union {epsilon}$, and $G$ has at most $abs(Pre union Suf) + 1 <= 2L + 1$ vertices.
] <thm:support>

#proof[
  Induct on decreasing length, proving both implications together. Let $s in.not Pre$. Then $s$ is not required. For each letter $d$ with $s d in V$, also $s d in.not Pre$, so induction gives $m(s d) = lambda(s d)$. Therefore
  $
    rho(s) = sum_d sum_c m(c s d) = sum_c sum_d m(c s d) <= sum_c m(c s) = lambda(s),
  $
  by the right extension rule at each $c s$. Let $(A, v, s, k)$ be a triggered rule with target $s$. Then $v in.not Pre$ and $abs(v) > abs(s)$, so $m(v) = lambda(v)$ by induction, and @lem:shift\(a) bounds the rule by $lambda(s)$. Every lower bound defining $m(s)$ is therefore at most $lambda(s)$, and $m(s) = lambda(s)$.

  Let $s in.not Suf$. Symmetrically, $lambda(s) <= rho(s)$, using induction on $c s in.not Suf$. For a triggered rule $(A, v, s, k)$, $s$ is a suffix of $v$, so $v in.not Suf$. Induction gives $m(v) = rho(v)$, and @lem:shift\(b) bounds the rule by $rho(s)$.

  For the consequences, $u(s) = m(s) - lambda(s)$ and $d(s) = m(s) - rho(s)$. An up edge $pref(s) -> s$ is used only if $s in Pre$, and then $pref(s) in Pre union {epsilon}$. Similarly, a down edge $s -> suf(s)$ is used only if $s in Suf$, and then $suf(s) in Suf union {epsilon}$.
]

#theorem(name: [Overlap-word restriction])[
  Let $m'$ be defined by the same descending recursion as $m$, but imposing (2.5) only for rules with $v in Ov$ and $w in Ov$. Then $m' = m$ on all of $V$; in particular, both define the same base graph.
] <thm:restrict>

#proof[
  Induct on decreasing length; assume $m' = m$ on all words longer than $s$. The quantities $lambda(s)$, $rho(s)$, $B_A (s;dot)$, and the triggers of all rules with target $s$ depend only on longer words, so they coincide for $m$ and $m'$. Since $m'(s)$ is the maximum of a subset of the bounds defining $m(s)$, we have $m'(s) <= m(s)$. Conversely, consider a triggered rule $(A, v, s, k)$ omitted from the restricted system, so $s in.not Ov$ or $v in.not Ov$. In either case $v in.not Ov$, because $s in.not Pre$ forces $v in.not Pre$ and $s in.not Suf$ forces $v in.not Suf$. If $v in.not Pre$, @thm:support gives $m(v) = lambda(v)$ and @lem:shift\(a) bounds the rule by $lambda(s) <= m'(s)$. If $v in.not Suf$, part (b) bounds it by $rho(s) <= m'(s)$. Thus $m(s) <= m'(s)$.
]

#corollary[
  Every count is determined by the counts on $Pre union Suf$: for $s in.not Pre$, $m(s) = sum_c m(c s)$, and for $s in.not Suf$, $m(s) = sum_c m(s c)$. The only period rules that must be evaluated are those with $v, w in Ov$, and, by @cor:mono, for each overlap word $w$ and each compatible periodic text $A$ only the largest triggered $k$.
] <cor:determined>

// ---------------------------------------------------------------------------
= Least periods do not suffice

The rule (2.5) ranges over all periods of $v$, not only its least period. Restricting the rule to least periods weakens the bound, even on overlap words.

#example[
  Let $cS = {tt("aabaa"), tt("baaab")}$, with optimum $7$ (`baaabaa`). The word $v = tt("aabaa") in Ov$ has least period $3$ and also period $4$. With $A = (tt("aaba"))^oo$, the rule $(A, v, tt("a"), 1)$ is triggered and gives $m(tt("a")) >= B_A (tt("a");m) + 2 = 2 + 2 = 4$. All other bounds on $m(tt("a"))$ are at most $3$. With all periods, $W = 6$; with least periods only, $m(tt("a")) = 3$ and $W = 5$. The base-graph lower bound is therefore genuinely weaker under the least-period restriction, and any efficient implementation must handle all periods of overlap words.
] <ex:least>

// ---------------------------------------------------------------------------
= Empirical confirmation and what remains

A direct implementation of the preprint's Section~2 confirmed every statement above, with no violations: @thm:support and both parts of @lem:shift (about $5 times 10^6$ triggered rules), @cor:mono, and @thm:restrict (identical base graphs on $4 times 10^5$ random, periodic, and Fibonacci-type instances).

The following observation is not proved here.

#conjecture[
  Without the period rule, $W = sum_c m(c)$ equals the weight of a minimum cycle cover of the overlap (distance) graph of $cS$.
]

#ind It held on all $1500$ instances tested. If true, the extension-only part of the construction is computable in $O(L)$ time by known Aho--Corasick techniques, and only the periodic correction is new.

#paragraph[Consequence for the algorithm.]
#cref2("Theorems", <thm:support>, <thm:restrict>) reduce the vertex set from $O(L^2)$ to $abs(Pre union Suf) <= 2L$, and the period rules to pairs of overlap words. On 600 reads of length 100 from a random genome this is $1.25 times 10^6$ substrings versus $1.1 times 10^5$ prefixes and suffixes and $2#h(0pt)","#h(0pt)879$ overlap words, with the number of $(v, p)$ rule sources falling from about $4 times 10^5$ to $944$. The blocking sums are handled in the next section.

// ---------------------------------------------------------------------------
= Blocking sums from the edge multiplicities <sec:blocking>

The blocking functional $B_A (w;m)$ ranges over bracketing words that need not lie in $Pre union Suf$. We now express it, and the trigger of a rule, using only $u$ and $d$, which by @thm:support live on $Pre$ and $Suf$ respectively and have total multiplicity $W$ each.

Throughout, $A$ is a periodic text with $A\[0:abs(x)\) = x$, and we write $x_n = A\[0:n\)$ for $n >= abs(x)$.

#lemma(name: [Left-bracket telescoping])[
  For a nonempty word $y$ whose left context is read in $A$ (that is, we put $y_mu = A\[-mu:0\) thin y$), define
  $
    Lambda^*_A (y) = sum_(mu >= 0) #h(1em/3) sum_(c != A(-mu - 1)) m(c thin y_mu),
  $
  the total count of words obtained from $y$ by extending along $A$ to the left and then adding one mismatching letter. Then $Lambda^*_A (y) = m(y) - sum_(mu >= 0) u(y_mu)$.
] <lem:left-tele>

#proof[
  For each $mu$, $sum_c m(c thin y_mu) = lambda(y_mu) = m(y_mu) - u(y_mu)$, and the term with $c = A(-mu - 1)$ is $m(y_(mu + 1))$. Hence the $mu$-th inner sum is $m(y_mu) - u(y_mu) - m(y_(mu + 1))$. Summing over $mu$ telescopes. The sum is finite, since $y_mu in.not V$ for large $mu$, and then all terms vanish.
]

#theorem(name: [Closed form for blocking sums])[
  Put
  $
    D_A (x) = sum_(n >= abs(x)) d(x_n), #h(2em)
    T_A (x) = sum_(n >= abs(x)) #h(1em/3) sum_(c != A(n)) #h(1em/3) sum_(mu >= 0)
    u bigp(A\[-mu:n\) thin c).
  $
  Then the unblocked surplus is
  #numeq($ U_A (x) = m(x) - B_A (x;m) = D_A (x) + T_A (x), $) <eq:U-closed>
  and equivalently
  #numeq($ B_A (x;m) = rho(x) - bigp(D_A (x) - d(x)) - T_A (x). $) <eq:B-closed>
] <thm:closed>

#proof[
  In @lem:decomp, the bracketing placements of $x_n$ whose right mismatch is adjacent are exactly the words $c' thin A\[-mu:0\) thin x_n thin c$ with $c' != A(-mu - 1)$ and $c != A(n)$, each with one placement. Hence, with equality, $R_A (x_n) = sum_(c != A(n)) Lambda^*_A (x_n c)$. Applying @lem:left-tele to $y = x_n c$, and using $sum_(c != A(n)) m(x_n c) = rho(x_n) - m(x_(n + 1)) = m(x_n) - d(x_n) - m(x_(n + 1))$,
  $
    R_A (x_n) = m(x_n) - m(x_(n + 1)) - d(x_n)
    - sum_(c != A(n)) sum_(mu >= 0) u bigp(A\[-mu:n\) thin c).
  $
  Iterating the right decomposition $B_A (x_n) = R_A (x_n) + B_A (x_(n + 1))$ from $n = abs(x)$ until $x_n in.not V$, where everything vanishes, gives $B_A (x;m) = m(x) - D_A (x) - T_A (x)$. Finally $m(x) - d(x) = rho(x)$.
]

#remark(name: [Walk interpretation])[
  On the closed walks of Sections~2--3 of the preprint, @eq:U-closed says that an occurrence of $x$ is unblocked exactly when, before any mismatch has entered on both sides, either the walk deletes the first letter of the occurrence while its window is still an $A$-aligned word $x_n$ (the term $D_A$), or the walk appends a first right mismatch $c$ while its window is an $A$-aligned word covering $x$ (the term $T_A$). This matches the proof of Lemma~3.4 of the preprint.
]

#corollary(name: [Counts from the supports of $u$ and $d$])[
  Process the words of $Pre union Suf$ in decreasing length. For each such word $s$,
  $
    lambda(s) = sum_(y" longer, ends with "s) u(y), #h(2em)
    rho(s) = sum_(y" longer, starts with "s) d(y),
  $
  by the telescoping identity in the proof of Lemma~3.1 of the preprint. Set $m(s) = rho(s)$ if $s in.not Suf$ and $m(s) = lambda(s)$ if $s in.not Pre$. If $s in Ov$, take the maximum of $lambda(s)$, $rho(s)$, the requirement indicator, and, for each pending rule $(A, k)$,
  $
    rho(s) + k + 1 - bigp(D_A (s) - d(s)) - T_A (s).
  $
  After fixing $m(s)$, set $u(s) = m(s) - lambda(s)$ and $d(s) = m(s) - rho(s)$. Then, for $s in Ov$ and each period $p$ of $s$, the rule with $A$ generated by $s\[0:p\)$ is triggered iff $D_A (s) + T_A (s) > 0$. Every quantity used involves $u$ and $d$ only on words already processed, together with $d(s)$ itself in the trigger.
] <cor:uv-only>

#ind In particular a rule can raise $m(w)$ above $rho(w)$ only if $k + 1 > (D_A (w) - d(w)) + T_A (w)$.

#lemma(name: [Primitive templates suffice])[
  If $v\[0:p\) = t^j$ with $j >= 2$, every rule $(A, v, w, k)$ with period $p$ is dominated by the rule $(A, v, w, j k)$ with period $abs(t)$.
] <lem:primitive>

#proof[
  Both rules use the same text $A = t^oo$, hence the same trigger and the same $B_A (w;m)$. Since $abs(v) - abs(w) = k p = (j k) abs(t)$, the second rule exists, and its bound $B_A (w;m) + j k + 1$ is larger.
]

#paragraph[Implementation and validation.]
A prototype (`compact.py`) keeps $u$ in a forward and a reversed weighted trie and $d$ in a forward trie, built incrementally in decreasing length. Then $lambda$ and $rho$ are subtree sums, and $D_A$ is a single walk along $A$. For $T_A$, words $A\[-mu:n\) c$ with the same start residue modulo $p$ are the same string, so $T_A (x)$ takes $p$ walks, each weighted by the number of admissible shifts. The prototype never builds $V$. On $4.6 times 10^5$ random, periodic and Fibonacci-type instances it reproduced the reference implementation's $u$ and $d$ exactly. It processes $2 times 10^4$ reads of length $100$ ($L = 2 times 10^6$) in under 90#h(1em/6)s in pure Python, whereas the reference construction would have to enumerate about $2 times 10^7$ distinct substrings there and evaluate blocking sums over them. The remaining costs are $O(abs(s))$ trie navigation per word and the $T_A$ walks. Both should become amortized constant time per step using Aho--Corasick node identifiers and Fenwick trees over Euler tours.

// ---------------------------------------------------------------------------
= The connection phase in practice <sec:practice>

The results above make the count construction practical. We now implement the remainder of the preprint (Sections~3--7: layers, group processing, cycle opening and the Euler tour) on top of the compact counts and compare the output with the classical greedy algorithm.

#paragraph[Implementation.]
The prototype (`connect.py`) follows the preprint step by step. It decomposes the base graph into closed walks and lifts each walk to windows of its periodic text. It then groups walks by primitive text and builds the ordered layers of Lemma~3.3 by sorting normalized exit ends, checking that the edge multiset is unchanged. Next it processes the groups as in Section~5, which covers incoming host requests (Lemma~4.3), the baseline, the easy cases, the threshold search and the record search, followed by the collective, individual and request cases. It then opens the cycles of planned links as in Section~6 and finally extracts the superstring by an Euler tour from $epsilon$. Every operation is materialized as an explicit closed walk of hierarchical-graph edges via the ordered-window construction of Lemma~4.1. Each inequality that the preprint proves for an operation is asserted at run time, as are balance of every added walk, the total budget $<= W$, and the containment of every input in the output. The substring set $V$ is never built. Word membership in $V$ is never queried either, because every prescribed window is guaranteed by the preprint to be an actual word.

#paragraph[Validation.]
On $6.4 times 10^4$ random, periodic and Fibonacci-type instances no assertion failed. On every instance with at most $7$ strings, the output was within a factor $2$ of the exact optimum, the worst ratio being $1.9375$. Coverage-guided mutation fuzzing (about $25$ minutes on $60$ cores, with no failures) exercised host requests, collective and individual blocks, easy band joins, and the opening of cycles of length at least $2$. Six branches were never reached: rooting layers below a baseline (both cases), self-cycles (both steps), records on a different phase of the same group, and hosts receiving requests from more than one child group. Their run-time checks are in place but untested.

#lemma(name: [Order-merge])[
  Let $T$ be a common superstring of a substring-free family $cS$. Order $cS$ by the position of its leftmost occurrence in $T$ and merge consecutive strings with maximum overlap. The result is a common superstring of length at most $abs(T)$.
] <lem:order-merge>

#proof[
  Let the leftmost occurrences be $[p_i, e_i)$ with $p_1 < dots.c < p_n$. Since $cS$ is substring-free, also $e_1 < dots.c < e_n$. If $e_i > p_(i + 1)$, the interval $[p_(i + 1), e_i)$ spells a proper suffix of $s_i$ that is a proper prefix of $s_(i + 1)$, so the maximum overlap $o_i$ is at least $e_i - p_(i + 1)$. Hence $abs(s_(i + 1)) - o_i <= e_(i + 1) - e_i$ in that case. Otherwise $abs(s_(i + 1)) - o_i <= e_(i + 1) - p_(i + 1) <= e_(i + 1) - e_i$. Summing, the merged length is at most $abs(s_1) + e_n - e_1 = e_n - p_1 <= abs(T)$.
]

#ind Applied to the output of the 2-approximation, order-merge therefore preserves the guarantee $abs(T) <= 2 W <= 2 "OPT"$. It costs $O(L)$ with an Aho--Corasick scan of $T$.

#paragraph[Experiments.]
The baseline is the maximum-overlap greedy algorithm in its efficient form (`greedy.py`). An Aho--Corasick automaton enumerates all suffix--prefix overlaps through failure links. The overlaps are then processed in decreasing length, with skip pointers over strings that already have a predecessor and a union--find structure that forbids cycles. On $2 times 10^4$ small instances its output length agreed with a naive pairwise-merge greedy except for tie-breaking. Because $W <= "OPT"$, the ratio $abs(T) \/ W$ certifies an upper bound on the true approximation ratio, and $abs(T) = W$ certifies optimality. @tab:exp lists representative large instances, and the full suite is in `experiments.py`.

#show figure.where(kind: table): it => { show figure.caption: set align(left); it }
#figure(
  kind: table,
  caption: [Output length relative to the lower bound $W$ (smaller is better; $1$ means provably optimal). Bold marks the shorter of ours+om and greedy when they differ. "Ours" is the raw Euler-tour output, "ours+om" adds order-merge. Genomes have length $2 times 10^4$; reads are error-free.],
  {
    set text(size: 9pt)
    table(
      columns: (auto, auto, auto, auto, auto, auto, auto),
      align: (left, left, right, right, right, right, right),
      stroke: none,
      inset: (x: 5pt, y: 2.6pt),
      table.hline(stroke: 0.8pt),
      table.header([family], [instance], [$n$], [$W$], [ours], [ours+om], [greedy]),
      table.hline(stroke: 0.5pt),
      [random-genome reads], [cov=1 l=50], [394], [12114], [1.0025], [1.0000], [1.0000],
      [], [cov=1 l=100], [198], [12033], [1.0018], [1.0001], [*1.0000*],
      [], [cov=3 l=50], [1163], [19010], [1.0025], [1.0000], [1.0000],
      [], [cov=3 l=100], [592], [18835], [1.0044], [1.0031], [*1.0000*],
      [], [cov=10 l=50], [3648], [19996], [1.0025], [1.0000], [1.0000],
      [], [cov=10 l=100], [1912], [19983], [1.0048], [1.0000], [1.0000],
      [repeat-rich reads], [s0 cov=3 l=100], [589], [16950], [1.0143], [*1.0031*], [1.0060],
      [], [s0 cov=10 l=100], [1816], [18023], [1.0159], [*1.0000*], [1.0034],
      [], [s1 cov=3 l=100], [571], [16533], [1.0240], [*1.0000*], [1.0018],
      [], [s1 cov=10 l=100], [1749], [17340], [1.0130], [*1.0000*], [1.0138],
      [], [s2 cov=3 l=100], [580], [16924], [1.0165], [1.0042], [*1.0008*],
      [], [s2 cov=10 l=100], [1784], [18094], [1.0150], [*1.0001*], [1.0032],
      [microsatellite reads], [unit=2 cov=5 l=40], [438], [3498], [1.0234], [1.0060], [*1.0040*],
      [], [unit=3 cov=5 l=40], [553], [4127], [1.0482], [*1.0148*], [1.0216],
      [], [unit=5 cov=5 l=40], [697], [4587], [1.1195], [*1.0275*], [1.0541],
      [random strings], [$|Sigma|$=2 n=300 l=12], [291], [1098], [1.0191], [1.0055], [*1.0000*],
      [], [$|Sigma|$=4 n=300 l=12], [300], [2505], [1.0040], [*1.0000*], [1.0016],
      [], [$|Sigma|$=2 n=1000 l=20], [1000], [9810], [1.0016], [1.0006], [*1.0000*],
      [greedy's bad family], [k=5, 20 copies], [60], [280], [1.1464], [*1.0000*], [1.5714],
      [], [k=20, 20 copies], [60], [880], [1.0466], [*1.0000*], [1.8636],
      [], [k=50, 20 copies], [60], [2080], [1.0197], [*1.0000*], [1.9423],
      table.hline(stroke: 0.8pt),
    )
  },
) <tab:exp>
#v(6pt)

#ind Over all $21$ large instances, ours+om was shorter than greedy on $11$, longer on $6$, and tied on $4$. It attained $W$, and was therefore provably optimal, on $11$ instances, against $8$ for greedy. The comparison by family is as follows.

- _Error-free reads from a random genome._ The base graph is nearly connected, and the connection phase adds at most $0.5%$ of $W$. Greedy is optimal or near-optimal here and slightly ahead.
- _Repeat-rich genomes and microsatellites._ Ours+om is shorter than greedy on $7$ of $9$ instances and reaches $W$ on $3$ of them. These are exactly the periodic structures the count rule (2.5) is designed for.
- _Greedy's classical bad family_ ${c(a b)^k, (b a)^k, (a b)^k e}$, in $20$ copies over disjoint alphabets: greedy's ratio tends to $2$ ($1.94$ at $k = 50$), while ours+om is exactly optimal.
- _Small instances against the exact optimum_ ($200$ periodic and $200$ binary instances, $3$--$7$ strings). The raw output is wasteful, with mean ratio $1.15$ and $1.21$, because every connection is paid for by round trips through $epsilon$. After order-merge the mean ratios are $1.002$ and $1.013$, against $1.003$ and $1.006$ for greedy. Ours+om is optimal on $194$ and $168$ of the $200$ instances, greedy on $192$ and $180$.

#paragraph[Cost.]
The Python prototype is $10$--$80$ times slower than greedy, for example $11.9$#h(1em/6)s against $0.23$#h(1em/6)s on 1,816 repeat-rich reads. The main costs are the $O(abs(s))$ trie navigation of @cor:uv-only, the $T_A$ walks, and the record search of Section~5, which scans all $2 W$ layer windows for each hard case. All three admit standard indexed implementations.
