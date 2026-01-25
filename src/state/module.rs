pub type ModuleId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleType {
    SawOsc,
    SinOsc,
    SqrOsc,
    TriOsc, // Oscillators
    Lpf,
    Hpf,
    Bpf, // Filters
    AdsrEnv, // Envelopes
    Lfo,     // Modulation
    Delay,
    Reverb, // Effects
    Output, // Output
}

impl ModuleType {
    pub fn name(&self) -> &'static str {
        match self {
            ModuleType::SawOsc => "Saw Oscillator",
            ModuleType::SinOsc => "Sine Oscillator",
            ModuleType::SqrOsc => "Square Oscillator",
            ModuleType::TriOsc => "Triangle Oscillator",
            ModuleType::Lpf => "Low-Pass Filter",
            ModuleType::Hpf => "High-Pass Filter",
            ModuleType::Bpf => "Band-Pass Filter",
            ModuleType::AdsrEnv => "ADSR Envelope",
            ModuleType::Lfo => "LFO",
            ModuleType::Delay => "Delay",
            ModuleType::Reverb => "Reverb",
            ModuleType::Output => "Output",
        }
    }

    pub fn default_params(&self) -> Vec<Param> {
        match self {
            ModuleType::SawOsc | ModuleType::SinOsc | ModuleType::SqrOsc | ModuleType::TriOsc => {
                vec![
                    Param {
                        name: "freq",
                        value: ParamValue::Float(440.0),
                        min: 20.0,
                        max: 20000.0,
                    },
                    Param {
                        name: "amp",
                        value: ParamValue::Float(0.5),
                        min: 0.0,
                        max: 1.0,
                    },
                ]
            }
            ModuleType::Lpf | ModuleType::Hpf | ModuleType::Bpf => vec![
                Param {
                    name: "cutoff",
                    value: ParamValue::Float(1000.0),
                    min: 20.0,
                    max: 20000.0,
                },
                Param {
                    name: "resonance",
                    value: ParamValue::Float(0.5),
                    min: 0.0,
                    max: 1.0,
                },
            ],
            ModuleType::AdsrEnv => vec![
                Param {
                    name: "attack",
                    value: ParamValue::Float(0.01),
                    min: 0.0,
                    max: 5.0,
                },
                Param {
                    name: "decay",
                    value: ParamValue::Float(0.1),
                    min: 0.0,
                    max: 5.0,
                },
                Param {
                    name: "sustain",
                    value: ParamValue::Float(0.7),
                    min: 0.0,
                    max: 1.0,
                },
                Param {
                    name: "release",
                    value: ParamValue::Float(0.3),
                    min: 0.0,
                    max: 10.0,
                },
            ],
            ModuleType::Lfo => vec![
                Param {
                    name: "rate",
                    value: ParamValue::Float(1.0),
                    min: 0.01,
                    max: 100.0,
                },
                Param {
                    name: "depth",
                    value: ParamValue::Float(0.5),
                    min: 0.0,
                    max: 1.0,
                },
            ],
            ModuleType::Delay => vec![
                Param {
                    name: "time",
                    value: ParamValue::Float(0.3),
                    min: 0.0,
                    max: 2.0,
                },
                Param {
                    name: "feedback",
                    value: ParamValue::Float(0.5),
                    min: 0.0,
                    max: 1.0,
                },
                Param {
                    name: "mix",
                    value: ParamValue::Float(0.3),
                    min: 0.0,
                    max: 1.0,
                },
            ],
            ModuleType::Reverb => vec![
                Param {
                    name: "room",
                    value: ParamValue::Float(0.5),
                    min: 0.0,
                    max: 1.0,
                },
                Param {
                    name: "damp",
                    value: ParamValue::Float(0.5),
                    min: 0.0,
                    max: 1.0,
                },
                Param {
                    name: "mix",
                    value: ParamValue::Float(0.3),
                    min: 0.0,
                    max: 1.0,
                },
            ],
            ModuleType::Output => vec![Param {
                name: "gain",
                value: ParamValue::Float(1.0),
                min: 0.0,
                max: 2.0,
            }],
        }
    }

    pub fn all_types() -> Vec<ModuleType> {
        vec![
            ModuleType::SawOsc,
            ModuleType::SinOsc,
            ModuleType::SqrOsc,
            ModuleType::TriOsc,
            ModuleType::Lpf,
            ModuleType::Hpf,
            ModuleType::Bpf,
            ModuleType::AdsrEnv,
            ModuleType::Lfo,
            ModuleType::Delay,
            ModuleType::Reverb,
            ModuleType::Output,
        ]
    }

    fn short_name(&self) -> &'static str {
        match self {
            ModuleType::SawOsc => "saw",
            ModuleType::SinOsc => "sin",
            ModuleType::SqrOsc => "sqr",
            ModuleType::TriOsc => "tri",
            ModuleType::Lpf => "lpf",
            ModuleType::Hpf => "hpf",
            ModuleType::Bpf => "bpf",
            ModuleType::AdsrEnv => "adsr",
            ModuleType::Lfo => "lfo",
            ModuleType::Delay => "delay",
            ModuleType::Reverb => "reverb",
            ModuleType::Output => "output",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: &'static str,
    pub value: ParamValue,
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone)]
pub enum ParamValue {
    Float(f32),
    Int(i32),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct Module {
    pub id: ModuleId,
    pub module_type: ModuleType,
    pub name: String,
    pub params: Vec<Param>,
}

impl Module {
    pub fn new(id: ModuleId, module_type: ModuleType) -> Self {
        let params = module_type.default_params();
        let name = format!("{}-{}", module_type.short_name(), id);

        Self {
            id,
            module_type,
            name,
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        let module = Module::new(1, ModuleType::SawOsc);
        assert_eq!(module.id, 1);
        assert_eq!(module.module_type, ModuleType::SawOsc);
        assert_eq!(module.name, "saw-1");
        assert_eq!(module.params.len(), 2); // freq and amp
    }

    #[test]
    fn test_oscillator_default_params() {
        let module = Module::new(0, ModuleType::SinOsc);
        assert_eq!(module.params.len(), 2);
        assert_eq!(module.params[0].name, "freq");
        assert_eq!(module.params[1].name, "amp");

        if let ParamValue::Float(freq) = module.params[0].value {
            assert_eq!(freq, 440.0);
        } else {
            panic!("Expected Float value for freq");
        }

        if let ParamValue::Float(amp) = module.params[1].value {
            assert_eq!(amp, 0.5);
        } else {
            panic!("Expected Float value for amp");
        }
    }

    #[test]
    fn test_filter_default_params() {
        let module = Module::new(0, ModuleType::Lpf);
        assert_eq!(module.params.len(), 2);
        assert_eq!(module.params[0].name, "cutoff");
        assert_eq!(module.params[1].name, "resonance");

        if let ParamValue::Float(cutoff) = module.params[0].value {
            assert_eq!(cutoff, 1000.0);
        } else {
            panic!("Expected Float value for cutoff");
        }
    }

    #[test]
    fn test_adsr_default_params() {
        let module = Module::new(0, ModuleType::AdsrEnv);
        assert_eq!(module.params.len(), 4);
        assert_eq!(module.params[0].name, "attack");
        assert_eq!(module.params[1].name, "decay");
        assert_eq!(module.params[2].name, "sustain");
        assert_eq!(module.params[3].name, "release");
    }

    #[test]
    fn test_all_module_types() {
        let types = ModuleType::all_types();
        assert_eq!(types.len(), 12);
        assert!(types.contains(&ModuleType::SawOsc));
        assert!(types.contains(&ModuleType::Output));
    }

    #[test]
    fn test_module_type_names() {
        assert_eq!(ModuleType::SawOsc.name(), "Saw Oscillator");
        assert_eq!(ModuleType::Lpf.name(), "Low-Pass Filter");
        assert_eq!(ModuleType::AdsrEnv.name(), "ADSR Envelope");
    }

    #[test]
    fn test_module_naming() {
        let module1 = Module::new(1, ModuleType::SawOsc);
        let module2 = Module::new(2, ModuleType::Lpf);
        assert_eq!(module1.name, "saw-1");
        assert_eq!(module2.name, "lpf-2");
    }
}
