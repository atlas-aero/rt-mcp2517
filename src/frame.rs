use embedded_can::{Frame,Id};
#[derive(Debug,Copy,Clone)]
pub struct CanFrame{
    pub identifier: Id,
    pub rtr: bool,
    pub dlc: usize,
    pub data: [u8;8],
}

impl Frame for CanFrame {
    fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
        if data.len()>8{
            return None;
        }
        
        let mut frame = CanFrame{
            identifier:id.into(),
            rtr: false,
            dlc: data.len(),
            data: [0;8],
        };
        frame.data[..data.len()].copy_from_slice(data);
        Some(frame)
    }
    fn new_remote(id: impl Into<Id>, dlc: usize) -> Option<Self> {
        if dlc>8{
            return None;
        }

        Some( CanFrame{
            identifier:id.into(),
            rtr: true,
            dlc,
            data: [0;8],
        })
    }
    fn is_extended(&self) -> bool {
        matches!(self.identifier,Id::Extended(_))
    }
    fn is_remote_frame(&self) -> bool {
        self.rtr
    }
    fn id(&self) -> Id {
        self.id()
    }
    fn dlc(&self) -> usize {
        self.dlc
    }
    fn data(&self) -> &[u8] {
        &self.data[..self.dlc()]
    }
}