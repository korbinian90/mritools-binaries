//! Provenance records for the command line tools.
//!
//! Mirrors `MriResearchTools.write_provenance` in the Julia implementation these
//! binaries are ported from, so a result produced here carries the same record as
//! one produced there: what was run (`settings_<tool>.txt`) and what has to be
//! cited for the methods that actually ran (`citations_<tool>.txt`).
//!
//! Citations are keyed by method and selected by the execution path. A citation
//! for a method the user did not run is as wrong as a missing one.

use std::fmt::Write as _;

/// A method the tools can use, for selecting citations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Romeo,
    Aspire,
    ClearSwi,
    Homogeneity,
    Laplacian,
    Bestpath,
    Tgv,
    TgvOriginal,
    PhaseBasedMasking,
    QsmxT,
}

impl Method {
    /// The reference text for this method.
    pub fn citation(self) -> &'static str {
        match self {
            Method::Romeo => "Dymerska, B., Eckstein, K., Bachrata, B., Siow, B., Trattnig, S., Shmueli, K., Robinson, S.D., 2020.\nPhase Unwrapping with a Rapid Opensource Minimum Spanning TreE AlgOrithm (ROMEO).\nMagnetic Resonance in Medicine.\nhttps://doi.org/10.1002/mrm.28563",
            Method::Aspire => "Eckstein, K., Dymerska, B., Bachrata, B., Bogner, W., Poljanc, K., Trattnig, S., Robinson, S.D., 2018.\nComputationally Efficient Combination of Multi-channel Phase Data From Multi-echo Acquisitions (ASPIRE).\nMagnetic Resonance in Medicine 79, 2996-3006.\nhttps://doi.org/10.1002/mrm.26963",
            Method::ClearSwi => "Eckstein, K., Bachrata, B., Hangel, G., Widhalm, G., Enzinger, C., Barth, M., Trattnig, S., Robinson, S.D., 2021.\nImproved susceptibility weighted imaging at ultra-high field using bipolar multi-echo acquisition and optimized image processing: CLEAR-SWI.\nNeuroImage 237, 118175.\nhttps://doi.org/10.1016/j.neuroimage.2021.118175",
            Method::Homogeneity => "Eckstein, K., Trattnig, S., Robinson, S.D., 2019.\nA Simple Homogeneity Correction for Neuroimaging at 7T.\nProceedings of the 27th Annual Meeting ISMRM. Presented at the ISMRM, Montreal, Quebec, Canada.\nhttps://index.mirasmart.com/ISMRM2019/PDFfiles/2716.html",
            Method::Laplacian => "Schofield, M.A., Zhu, Y., 2003.\nFast phase unwrapping algorithm for interferometric applications.\nOptics Letters 28, 1194-1196.\nhttps://doi.org/10.1364/OL.28.001194",
            Method::Bestpath => "Abdul-Rahman, H.S., Gdeisat, M.A., Burton, D.R., Lalor, M.J., Lilley, F., Moore, C.J., 2007.\nFast and robust three-dimensional best path phase unwrapping algorithm.\nApplied Optics 46, 6623-6635.\nhttps://doi.org/10.1364/AO.46.006623",
            Method::Tgv => "Langkammer, C., Bredies, K., Poser, B.A., Barth, M., Reishofer, G., Fan, A.P., Bilgic, B., Fazekas, F., Mainero, C., Ropele, S., 2015.\nFast quantitative susceptibility mapping using 3D EPI and total generalized variation.\nNeuroImage 111, 622-630.\nhttps://doi.org/10.1016/j.neuroimage.2015.02.041",
            Method::TgvOriginal => "Bredies, K., Ropele, S., Poser, B.A., Barth, M., Langkammer, C., 2014.\nSingle-step quantitative susceptibility mapping using total generalized variation and 3D EPI.\nProceedings of the 22nd Annual Meeting ISMRM, p. 604.",
            Method::PhaseBasedMasking => "Hagberg, G.E., Eckstein, K., Tuzzi, E., Zhou, J., Robinson, S.D., Scheffler, K., 2022.\nPhase-based masking for quantitative susceptibility mapping of the human brain at 9.4T.\nMagnetic Resonance in Medicine.\nhttps://doi.org/10.1002/mrm.29368",
            Method::QsmxT => "Stewart, A.W., Robinson, S.D., O'Brien, K., Jin, J., Widhalm, G., Hangel, G., Walls, A., Goodwin, J., Eckstein, K., Tourell, M., Morgan, C., Narayanan, A., Barth, M., Bollmann, S., 2022.\nQSMxT: Robust masking and artifact reduction for quantitative susceptibility mapping.\nMagnetic Resonance in Medicine.\nhttps://doi.org/10.1002/mrm.29048",
        }
    }

    /// A non-citation notice this method carries, if any. Written into the
    /// citations file when the method ran, because that is the file someone
    /// reads before publishing or before shipping a product.
    pub fn notice(self) -> Option<&'static str> {
        match self {
            Method::Aspire => Some(
                "PATENT: MCPC-3D-S / ASPIRE is covered by US10605885B2\n\
                 (https://patents.google.com/patent/US10605885B2/en). Per the upstream ASPIRE\n\
                 repository, no licence is required for scientific use and the method can be\n\
                 applied free of charge, but a licence IS required for commercial use, and the\n\
                 method is not a medical product, so it may not be used for diagnosis in humans.\n\
                 Note that an MIT licence grants copyright permissions only, not patent rights.",
            ),
            _ => None,
        }
    }
}

/// What a run recorded about itself.
pub struct Provenance<'a> {
    pub tool: &'a str,
    pub args: &'a [String],
    /// Resolved settings as `name => value`, including the ones derived at run
    /// time. Echo times in particular: the raw argument alone does not say what
    /// a range like `3.5:3.5:14` expanded to.
    pub settings: Vec<(String, String)>,
    /// `name => path` for the inputs that were read.
    pub inputs: Vec<(String, String)>,
    /// Methods that actually ran.
    pub cite: Vec<Method>,
    /// Further reading, listed separately from what ran.
    pub optional: Vec<Method>,
}

impl<'a> Provenance<'a> {
    pub fn new(tool: &'a str, args: &'a [String]) -> Self {
        Self {
            tool,
            args,
            settings: Vec::new(),
            inputs: Vec::new(),
            cite: Vec::new(),
            optional: Vec::new(),
        }
    }

    pub fn setting(mut self, name: &str, value: impl std::fmt::Display) -> Self {
        self.settings.push((name.to_string(), value.to_string()));
        self
    }

    pub fn input(mut self, name: &str, path: &str) -> Self {
        self.inputs.push((name.to_string(), path.to_string()));
        self
    }

    pub fn used(mut self, m: Method) -> Self {
        if !self.cite.contains(&m) {
            self.cite.push(m);
        }
        self
    }

    pub fn optional(mut self, m: Method) -> Self {
        self.optional.push(m);
        self
    }

    /// Write `settings_<tool>.txt` and `citations_<tool>.txt` into `dir`.
    pub fn write(&self, dir: &str) -> anyhow::Result<()> {
        let base = std::path::Path::new(dir);

        let mut s = String::new();
        writeln!(
            s,
            "# {} {} (Rust port)",
            self.tool,
            env!("CARGO_PKG_VERSION")
        )?;
        writeln!(
            s,
            "# NOTE: this is the Rust port. It is not yet numerically equivalent to the"
        )?;
        writeln!(
            s,
            "#       Julia implementation it is derived from - see the parity table in"
        )?;
        writeln!(
            s,
            "#       the mritools-binaries README before comparing or pooling results."
        )?;
        writeln!(s, "# command: {}", self.args.join(" "))?;
        writeln!(s)?;
        writeln!(s, "[versions]")?;
        writeln!(s, "mritools-binaries: {}", env!("CARGO_PKG_VERSION"))?;
        writeln!(s)?;
        if !self.inputs.is_empty() {
            writeln!(s, "[inputs]")?;
            for (name, path) in &self.inputs {
                let abs = std::fs::canonicalize(path)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| path.clone());
                writeln!(s, "{}: {}", name, abs)?;
            }
            writeln!(s)?;
        }
        writeln!(s, "[settings]")?;
        let mut sorted = self.settings.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, value) in &sorted {
            writeln!(s, "{}: {}", name, value)?;
        }
        std::fs::write(base.join(format!("settings_{}.txt", self.tool)), s)?;

        let mut c = String::new();
        writeln!(c, "# Citations for the methods this run actually used.")?;
        writeln!(
            c,
            "# Methods that were available but not used are deliberately absent."
        )?;
        writeln!(c)?;
        for m in &self.cite {
            writeln!(c, "{}\n", m.citation())?;
        }
        let notices: Vec<_> = self.cite.iter().filter_map(|m| m.notice()).collect();
        if !notices.is_empty() {
            writeln!(c, "# Notices for the methods used:")?;
            writeln!(c)?;
            for n in notices {
                writeln!(c, "{}\n", n)?;
            }
        }
        let optional: Vec<_> = self
            .optional
            .iter()
            .filter(|m| !self.cite.contains(m))
            .collect();
        if !optional.is_empty() {
            writeln!(c, "# Optional citations:")?;
            writeln!(c)?;
            for m in optional {
                writeln!(c, "{}\n", m.citation())?;
            }
        }
        std::fs::write(base.join(format!("citations_{}.txt", self.tool)), c)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args() -> Vec<String> {
        vec!["romeo".into(), "-p".into(), "p.nii".into()]
    }

    #[test]
    fn records_settings_and_command() {
        let dir = tempfile::tempdir().unwrap();
        let a = args();
        Provenance::new("romeo", &a)
            .setting("echo_times", "[4, 8, 12]")
            .setting("weights", "romeo3")
            .used(Method::Romeo)
            .write(dir.path().to_str().unwrap())
            .unwrap();
        let s = std::fs::read_to_string(dir.path().join("settings_romeo.txt")).unwrap();
        assert!(s.contains("# command: romeo -p p.nii"));
        // Echo times must survive into the record: the raw argument alone does
        // not say what a range expanded to.
        assert!(s.contains("echo_times: [4, 8, 12]"));
        assert!(s.contains("weights: romeo3"));
        // The record has to say this is the port, not the reference.
        assert!(s.contains("not yet numerically equivalent"));
    }

    #[test]
    fn settings_are_sorted_for_stable_diffs() {
        let dir = tempfile::tempdir().unwrap();
        let a = args();
        Provenance::new("t", &a)
            .setting("zulu", 1)
            .setting("alpha", 2)
            .used(Method::Romeo)
            .write(dir.path().to_str().unwrap())
            .unwrap();
        let s = std::fs::read_to_string(dir.path().join("settings_t.txt")).unwrap();
        assert!(s.find("alpha").unwrap() < s.find("zulu").unwrap());
    }

    #[test]
    fn cites_only_what_ran() {
        let dir = tempfile::tempdir().unwrap();
        let a = args();
        Provenance::new("romeo", &a)
            .used(Method::Romeo)
            .optional(Method::QsmxT)
            .write(dir.path().to_str().unwrap())
            .unwrap();
        let c = std::fs::read_to_string(dir.path().join("citations_romeo.txt")).unwrap();
        assert!(c.contains("Rapid Opensource Minimum Spanning"));
        assert!(
            !c.contains("ASPIRE"),
            "must not cite a method that did not run"
        );
        assert!(c.contains("# Optional citations:"));
        assert!(c.contains("QSMxT"));
    }

    #[test]
    fn aspire_carries_its_patent_notice_once() {
        let dir = tempfile::tempdir().unwrap();
        let a = args();
        Provenance::new("mcpc3ds", &a)
            .used(Method::Aspire)
            .used(Method::Aspire) // deduplicated
            .write(dir.path().to_str().unwrap())
            .unwrap();
        let c = std::fs::read_to_string(dir.path().join("citations_mcpc3ds.txt")).unwrap();
        assert_eq!(
            c.matches("Computationally Efficient Combination").count(),
            1
        );
        assert!(c.contains("US10605885B2"));
        assert!(c.contains("may not be used for diagnosis in humans"));
    }

    #[test]
    fn no_patent_notice_when_the_method_did_not_run() {
        let dir = tempfile::tempdir().unwrap();
        let a = args();
        Provenance::new("romeo", &a)
            .used(Method::Romeo)
            .write(dir.path().to_str().unwrap())
            .unwrap();
        let c = std::fs::read_to_string(dir.path().join("citations_romeo.txt")).unwrap();
        assert!(!c.contains("US10605885B2"));
    }
}
