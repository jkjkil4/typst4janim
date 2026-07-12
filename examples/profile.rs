use std::thread;
use std::{collections::HashMap, time::Duration};

use pyo3::prelude::*;

use typst4janim::typst4janim::compile;

/// Commands:
/// - `cargo build --example profile --profile profiling`
/// - `samply record ./target/profiling/examples/profile`
fn main() {
    Python::initialize();
    Python::attach(|py| {
        let compile = |input| {
            let result = compile(py, input, HashMap::new(), None, None);
            match result {
                Ok(collected) => {
                    println!("Collected: {:?}", collected);
                }
                Err(err) => {
                    println!("Err: {}", err);
                }
            }
        };

        thread::sleep(Duration::from_millis(50));

        // Hello
        compile(
            r#"
                Hello
            "#
            .into(),
        );

        thread::sleep(Duration::from_millis(50));

        // World
        compile(
            r#"
                World
            "#
            .into(),
        );

        thread::sleep(Duration::from_millis(50));

        // Lorem 200
        compile(
            r#"
                #lorem(200)
            "#
            .into(),
        );

        thread::sleep(Duration::from_millis(50));

        // CeTZ
        compile(
            r#"
                #import "@preview/cetz:0.4.2"
                #import "@preview/cetz-plot:0.1.3": *

                #let angle = 30;
                #let width = 1
                #let ang_deg = angle * 1deg

                #cetz.canvas({
                    import cetz.draw: *

                    stroke((thickness: 0.7pt, join: "round", paint: white))

                    let (a, b, c, d) = (
                        (0, 0),
                        (width, 0),
                        (rel: (width, 0), to: (60deg, width * 3)),
                        (60deg, width * 3),
                    )

                    line(a, b, c, d, a)

                    let ang_eab = ang_deg
                    let len_ae = width / calc.sin(60deg - ang_eab) * calc.sin(120deg)
                    let e = (ang_eab, len_ae)
                    let g = (a, 100%, 120deg, e)
                    let f = (a, 100%, 60deg, e)

                    line(a, e, f, g, a)
                    line(a, f)

                    for (pos, rel, lab) in (
                        (a, (-1, -1.2), $A$),
                        (b, (1, -1.5), $B$),
                        (c, (1, 1), $C$),
                        (d, (-1, 1), $D$),
                        (f, (-.5, 1.5), $F$),
                        (g, (-1, 1), $G$),
                        (e, (1, -.5), $E$),
                    ) {
                        content((pos, 17%, (rel: rel)), lab)
                    }
                })
            "#
            .into(),
        );

        thread::sleep(Duration::from_millis(50));

        // CeTZ
        compile(
            r#"
                #import "@preview/cetz:0.4.2"
                #import "@preview/cetz-plot:0.1.3": *

                #let angle = 31;
                #let width = 1
                #let ang_deg = angle * 1deg

                #cetz.canvas({
                    import cetz.draw: *

                    stroke((thickness: 0.7pt, join: "round", paint: white))

                    let (a, b, c, d) = (
                        (0, 0),
                        (width, 0),
                        (rel: (width, 0), to: (60deg, width * 3)),
                        (60deg, width * 3),
                    )

                    line(a, b, c, d, a)

                    let ang_eab = ang_deg
                    let len_ae = width / calc.sin(60deg - ang_eab) * calc.sin(120deg)
                    let e = (ang_eab, len_ae)
                    let g = (a, 100%, 120deg, e)
                    let f = (a, 100%, 60deg, e)

                    line(a, e, f, g, a)
                    line(a, f)

                    for (pos, rel, lab) in (
                        (a, (-1, -1.2), $A$),
                        (b, (1, -1.5), $B$),
                        (c, (1, 1), $C$),
                        (d, (-1, 1), $D$),
                        (f, (-.5, 1.5), $F$),
                        (g, (-1, 1), $G$),
                        (e, (1, -.5), $E$),
                    ) {
                        content((pos, 17%, (rel: rel)), lab)
                    }
                })
            "#
            .into(),
        );

        thread::sleep(Duration::from_millis(50));
    })
}
