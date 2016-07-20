use vec_map::VecMap;

#[derive(Debug)]
pub struct Mapping {
    axes: VecMap<u16>,
    btns: VecMap<u16>,
}

impl Mapping {
    pub fn new() -> Self {
        Mapping {
            axes: VecMap::new(),
            btns: VecMap::new(),
        }
    }

    pub fn map(&self, code: u16, kind: Kind) -> u16 {
        match kind {
            Kind::Button => *self.btns.get(code as usize).unwrap_or(&code),
            Kind::Axis => *self.axes.get(code as usize).unwrap_or(&code),
        }
    }

    pub fn map_rev(&self, code: u16, kind: Kind) -> u16 {
        match kind {
            Kind::Button => {
                self.btns
                    .iter()
                    .find(|x| *x.1 == code)
                    .unwrap_or((code as usize, &0))
                    .0 as u16
            }
            Kind::Axis => {
                self.axes.iter().find(|x| *x.1 == code).unwrap_or((code as usize, &0)).0 as u16
            }
        }
    }
}

pub enum Kind {
    Button,
    Axis,
}
