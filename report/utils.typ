#import "@preview/cetz:0.4.1"
#import "@preview/ctheorems:1.1.3": *

#let hole_outline(x, y) = {
  cetz.draw.circle((x,y), radius: 0.35, stroke: 0.5pt)
}
#let peg(x, y) = {
  cetz.draw.circle((x, y), radius: 0.28, fill: oklab(20%, 0, 0), stroke: none)
}
#let triple_two(x, y) = {
  hole_outline(x, y)
  peg(x, y)
  hole_outline(x + 1, y)
  peg(x + 1, y)
  hole_outline(x + 2, y)
}
#let triple_one(x, y) = {
  hole_outline(x, y)
  hole_outline(x + 1, y)
  hole_outline(x + 2, y)
  peg(x + 2, y)
}

#let timearrow_r = text(weight: 100, 18pt, sym.arrow.r.double)

#let board(pattern) = {
  import cetz.draw: *

  group({
    set-style(stroke: oklab(40%, 0, 0) + 2.0pt)
    // circle((0, 0), radius: 4.2)

    let cross_padding = 0.7
    let outer_radius = cross_padding
    let inner_radius = 1 - cross_padding

    let far = 3 + cross_padding
    let near = 1 + cross_padding

    merge-path(fill: white,  {
      for angle in (0, 1, 2, 3) {
        rotate(z: 90deg)
        line((far, -near + outer_radius), (far, near - outer_radius))
        arc((far - 0 * outer_radius, near - outer_radius), start: 0deg, stop: 90deg, radius: outer_radius)
        line((far - outer_radius, near), (near + inner_radius, near))
        arc((near + inner_radius, near), start: 270deg, stop: 180deg, radius: inner_radius) 
        line((near, near + inner_radius), (near, far - outer_radius))
        arc((near, far - outer_radius), start: 0deg, stop: 90deg, radius: outer_radius) 
      }
    })
  })

  for x in (-3, -2, -1, 0, 1, 2, 3) {
    for y in (-3, -2, -1, 0, 1, 2, 3) {
      if calc.abs(x) <= 1 or calc.abs(y) <= 1 {
        hole_outline(x, y)

        if pattern == "start" {
          if x != 0 or y != 0 {
            peg(x, y)
          }
        } else if pattern == "mid" {
          if (x < 2 and y == 1) or (x == 0 and y <= 1 and y > -3) or (x < 3 and x != -1 and y == 0) or (x < 2 and x != -2 and y == -1) or (x == -1 and y == 2) {
            peg(x, y)
          }

        } else {
          if x == 0 and y == 0 {
            peg(x, y)
          }
        }
      }

    }

  }
}

#let shortline(col) = [#box(rect(width: 8pt, height: 1.7pt, fill: col), baseline: -2pt)#sym.space.nobreak]

#show: thmrules.with(qed-symbol: $square$)

#let thm-padding = 1.5em
#let theorem = thmplain(
  "theorem",
  "Theorem",
  base: none,
  titlefmt: strong,
  inset: (top: 1.5em, left: thm-padding, right: thm-padding),
)
#let proof = thmproof(
  "proof",
  "Proof",
  inset: (top: 1em, bottom: 1.5em, left: thm-padding, right: thm-padding),
)
