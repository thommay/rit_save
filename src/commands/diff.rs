use crate::diff::hunk::Hunk;
use colored::Colorize;
    //    pager();
            .unwrap_or_else(|| String::from(NILL_OID));
        println!(
            "{}",
            format!("diff --git {} {}", a_pth_str, b_pth_str).bold()
        );
            println!("{}", format!("new file mode {}", b.mode.unwrap()).bold());
            println!(
                "{}",
                format!("deleted file mode {}", a.mode.unwrap()).bold()
            );
            println!("{}", format!("old mode {}", a.mode.unwrap()).bold());
            println!("{}", format!("new mode {}", b.mode.unwrap()).bold());
            format!(" {}", &a.mode.unwrap().bold())
        println!(
            "{}",
            format!("index {}..{}{}", a.oid, b.oid, mode_str).bold()
        );
        println!("{}", format!("--- {}", a_pth_str).bold());
        println!("{}", format!("+++ {}", b_pth_str).bold());
        for hunk in Hunk::filter(edits) {
            println!("{}", hunk.header().cyan());
            for edit in hunk.edits {
                println!("{}", edit);
            }