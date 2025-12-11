use crate::pci::vpci_dev::standard::mmio_vdev_standard_handler;
use crate::percpu::this_zone;
use crate::{error::HvResult, pci::pci_struct::VirtualPciConfigSpace};
use crate::pci::pci_struct::PciConfigSpace;
use crate::pci::pci_access::EndpointField;
use crate::pci::PciConfigAddress;
use super::{PciConfigAccessStatus, VpciDeviceHandler};
use crate::memory::frame::Frame;
use crate::pci::vpci_dev::Bar;
use crate::memory::MMIOAccess;
/*
0000000 1af4 1044 0406 0010 0001 00ff 0008 0000
0000010 0000 0000 1000 1004 0000 0000 0000 0000
0000020 400c 0000 0080 0000 0000 0000 0000 0000
0000030 0000 0000 0098 0000 0000 0000 0127 0000
0000040 0009 0110 0004 0000 0000 0000 1000 0000
0000050 4009 0310 0004 0000 1000 0000 1000 0000
0000060 5009 0410 0004 0000 2000 0000 1000 0000
0000070 6009 0214 0004 0000 3000 0000 1000 0000
0000080 0004 0000 7009 0514 0000 0000 0000 0000
0000090 0000 0000 0000 0000 8411 8001 0001 0000
00000a0 0801 0000 0000 0000 0000 0000 0000 0000
00000b0 0000 0000 0000 0000 0000 0000 0000 0000
*
0000100

*/

pub(crate) const COPY_CSPACE_U32: [u16; 0x200 / 4] = [
    0x1af4,0x1044,0x0406,0x0010,0x0001,0x00ff,0x0008,0x0000,
    0x0000,0x0000,0x1000,0x1004,0x0000,0x0000,0x0000,0x0000,
    0x400c,0x0000,0x0080,0x0000,0x0000,0x0000,0x0000,0x0000,
    0x0000,0x0000,0x0098,0x0000,0x0000,0x0000,0x0127,0x0000,
    0x0009,0x0110,0x0004,0x0000,0x0000,0x0000,0x1000,0x0000,
    0x4009,0x0310,0x0004,0x0000,0x1000,0x0000,0x1000,0x0000,
    0x5009,0x0410,0x0004,0x0000,0x2000,0x0000,0x1000,0x0000,
    0x6009,0x0214,0x0004,0x0000,0x3000,0x0000,0x1000,0x0000,
    0x0004,0x0000,0x7009,0x0514,0x0000,0x0000,0x0000,0x0000,
    0x0000,0x0000,0x0000,0x0000,0x8411,0x8001,0x0001,0x0000,
    0x0801,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,
    0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,
    0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,
    0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,
    0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,
    0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,
    ];

const VIRTIO_RNG_VENDOR_ID: u16 = 0x1af4;
const VIRTIO_RNG_DEVICE_ID: u16 = 0x1044;
const PCI_STS_CAPS: u16 = 0x0010; // bit 4
const RNG_REVISION: u8 = 0x01; 
const PCI_DEV_CLASS_OTHER: u32 = 0x00ff0000;
const PCI_CFG_CAPS: usize = 0x34;
const PCI_CAP_ID_VNDR: u8 = 0x09;
const PCI_CAP_ID_MSIX: u8 = 0x11;
const RNG_CFG_VNDR_CAP: u8 = 0x98;
const CAP_UNKONWN_POS:u8 = 0x84;
const CAP_UNKONWN_ID:u8 = 0x09;
const CAP_UNKONWN_U16:u16 = 0x0514;
const CAP_NOTIFY_POS:u8 = 0x70;
const CAP_NOTIFY_ID:u8 = 0x09;
const CAP_NOTIFY_U16:u16 = 0x0214;
const CAP_DEVICECFG_POS:u8 = 0x60;
const CAP_DEVICECFG_U16:u16 = 0x0410;
const CAP_DEVICECFG_ID:u8 = 0x09;
const CAP_ISR_POS:u8 = 0x50;
const CAP_ISR_U16:u16 = 0x0310;
const CAP_ISR_ID:u8 = 0x09;
const CAP_COMMONCFG_POS:u8 = 0x40;
const CAP_COMMONCFG_U16:u16 = 0x0110;
const CAP_COMMONCFG_ID:u8 = 0x09;
const CAP_MSIX_POS:u8 = 0x98;
const CAP_MSIX_ID:u8 = 0x11;
const CAP_MSIX_MSGCON:u16 = 8001;
// const STANDARD_CFG_VNDR_LEN: u8 = 0x20;
// const STANDARD_CFG_MSIX_CAP: usize = 0x60; // VNDR_CAP + VNDR_LEN
// const STANDARD_MSIX_VECTORS: u16 = 16;
const RNG_CFG_SIZE: usize = 0x100;

pub(crate) const DEFAULT_CSPACE_U32: [u32; RNG_CFG_SIZE / 4] = {
    let mut arr = [0u32; RNG_CFG_SIZE / 4];
    // DEVICE ID ----- VENDOR ID
    arr[0x00 / 4] = (VIRTIO_RNG_DEVICE_ID as u32) << 16 | VIRTIO_RNG_VENDOR_ID as u32;
    // Status ------ Command
    arr[0x04 / 4] = (PCI_STS_CAPS as u32) << 16;
    // Class ----- Revision ID
    arr[0x08 / 4] = PCI_DEV_CLASS_OTHER | (RNG_REVISION as u32);
    // Subsystem ID ----- Subsystem vendor ID
    arr[0x2c / 4] = (VIRTIO_RNG_DEVICE_ID as u32) << 16 | VIRTIO_RNG_VENDOR_ID as u32;
    // capability pointer = 0x98
    arr[PCI_CFG_CAPS / 4] = CAP_MSIX_POS as u32;
    // capability 0 = {id = MSIX;next_ptr = 0x84} 
    arr[CAP_MSIX_POS as usize / 4] = (CAP_MSIX_MSGCON as u32) << 16
        | (CAP_UNKONWN_POS as u32) << 8
        | CAP_MSIX_ID as u32;
    // capability 1 = {id = UNKONWN;next_ptr = 0x70}
    arr[CAP_UNKONWN_POS as usize / 4] = (CAP_UNKONWN_U16 as u32) << 16
        | (CAP_NOTIFY_POS as u32) << 8
        | (CAP_UNKONWN_ID as u32);
    // capability 2 = {id = NOTIFY;next_ptr = 0x60}
    arr[CAP_NOTIFY_POS as usize / 4] = (CAP_NOTIFY_U16 as u32) << 16
        | (CAP_DEVICECFG_POS as u32) << 8
        | (CAP_NOTIFY_ID as u32);
    // capability 3 = {id = DEVICECFG;next_ptr = 0x50}
    arr[CAP_DEVICECFG_POS as usize / 4] = (CAP_DEVICECFG_U16 as u32) << 16
        | (CAP_ISR_POS as u32) << 8
        | (CAP_DEVICECFG_ID as u32);
    // capability 4 = {id = ISR;next_ptr = 0x40}
    arr[CAP_ISR_POS as usize / 4] = (CAP_ISR_U16 as u32) << 16
        | (CAP_COMMONCFG_POS as u32) << 8
        | (CAP_ISR_ID as u32);
    // capability 5 = {id = COMMONCFG;next_ptr = 0x0}
    arr[CAP_COMMONCFG_POS as usize / 4] = (CAP_COMMONCFG_U16 as u32) << 16
        | (0x0) << 8
        | (CAP_COMMONCFG_ID as u32); 
    // arr[STANDARD_CFG_MSIX_CAP / 4] = (0x00u32) << 8 | PCI_CAP_ID_MSIX as u32;
    // arr[(STANDARD_CFG_MSIX_CAP + 0x4) / 4] = 1;
    // arr[(STANDARD_CFG_MSIX_CAP + 0x8) / 4] = ((0x10 * STANDARD_MSIX_VECTORS) as u32) | 1;
    arr
};

/// Handler for standard virtual PCI devices
pub struct VirtioRngHandler;

impl VpciDeviceHandler for VirtioRngHandler {

    fn init_bar(&self) -> crate::pci::pci_access::Bar {
        let mut bar = Bar::default();
        bar[0].set_size(0x0);
        bar[1].set_size(0x4000);
        bar[2].set_size(0x0);
        bar[3].set_size(0x0);
        bar[4].set_size(0x10000);
        bar[5].set_size(0x0);

        let frame1 = Frame::new().unwrap();
        let frame4 = Frame::new().unwrap();
        bar[1].set_value(frame1.start_paddr() as u64);
        bar[4].set_value(frame4.start_paddr() as u64);
        bar[1].config_bar(crate::pci::pci_access::PciMemType::Mem32, false);
        bar[4].config_bar(crate::pci::pci_access::PciMemType::Mem64Low, true);
        bar[5].config_bar(crate::pci::pci_access::PciMemType::Mem64High, true);
        // value is the paddr of the mem you allocate
        // let frame = Frame::new().unwrap();
        // let start = frame.start_paddr();
        // let size = frame.size();
        // bar[0].set_value(start as u64);
        // bar[0].set_virtual_value(start as u64);
        // bar[0].set_size(size as u64);
        info!("114514::init_bar:{:?},frame1:{:?},frame4:{:?}",bar,frame1,frame4);
        bar
    }

    fn read_cfg(&self, _space: &mut VirtualPciConfigSpace, offset: PciConfigAddress, size: usize) -> HvResult<PciConfigAccessStatus> {
        // info!("virt pci standard read_cfg, offset {:#x}, size {:#x}", offset, size);
        match EndpointField::from(offset as usize, size) {
            // EndpointField::ID => {
            //     Ok(PciConfigAccessStatus::Done(_space.get(EndpointField::ID) as usize))
            // }
            // EndpointField::CapabilityPointer =>{
            //     Ok(PciConfigAccessStatus::Done(_space.get(EndpointField::CapabilityPointer) as usize))
            // }
            EndpointField::Bar(n)=>{
                let bar = _space.get_bararr()[n];
                // return Ok(PciConfigAccessStatus::Done(0x0));
                if(bar.get_size_read()){
                    return Ok(PciConfigAccessStatus::Done(bar.get_size() as usize))
                }else{
                    return Ok(PciConfigAccessStatus::Perform)
                }
            }
            _ => {
                Ok(PciConfigAccessStatus::Perform)
            }
        }
    }

    fn write_cfg(&self, space: &mut VirtualPciConfigSpace, offset: PciConfigAddress, size: usize, value: usize) -> HvResult<PciConfigAccessStatus> {
        // info!("virt pci standard write_cfg, offset {:#x}, size {:#x}, value {:#x}", offset, size, value);
        match EndpointField::from(offset as usize, size) {
            EndpointField::ID => {
                Ok(PciConfigAccessStatus::Reject)
            }
            // EndpointField::Command => {
            //     space.set(EndpointField::Command, value as u32);
            //     Ok(PciConfigAccessStatus::Done(value))
            // }
            // EndpointField::Bar0=>{
            //     if(value == 0xffff_ffff){
            //         space.set(EndpointField::Bar0, 0xffff_f000);
            //         Ok(PciConfigAccessStatus::Done(value))
            //     }else{
            //         Ok(PciConfigAccessStatus::Perform)
            //     }
            // }
            // EndpointField::Bar1=>{
            //     if(value == 0xffff_ffff){
            //         space.set(EndpointField::Bar1, 0xffff_f000);
            //         Ok(PciConfigAccessStatus::Done(value))
            //     }else{
            //         Ok(PciConfigAccessStatus::Perform)
            //     }
            // }
            // EndpointField::Bar2=>{
            //     if(value == 0xffff_ffff){
            //         space.set(EndpointField::Bar2, 0xffff_f000);
            //         Ok(PciConfigAccessStatus::Done(value))
            //     }else{
            //         Ok(PciConfigAccessStatus::Perform)
            //     }
            // }
            // EndpointField::Bar3=>{
            //     if(value == 0xffff_ffff){
            //         space.set(EndpointField::Bar3, 0xffff_f000);
            //         Ok(PciConfigAccessStatus::Done(value))
            //     }else{
            //         Ok(PciConfigAccessStatus::Perform)
            //     }
            // }
            // EndpointField::Bar4=>{
            //     if(value == 0xffff_ffff){
            //         space.set(EndpointField::Bar4, 0xffff_c000);
            //         Ok(PciConfigAccessStatus::Done(value))
            //     }else{
            //         Ok(PciConfigAccessStatus::Perform)
            //     }
            // }
            // EndpointField::Bar5=>{
            //     if(value == 0xffff_ffff){
            //         space.set(EndpointField::Bar5, 0xffff_f000);
            //         Ok(PciConfigAccessStatus::Done(value))
            //     }else{
            //         Ok(PciConfigAccessStatus::Perform)
            //     }
            // }
            EndpointField::Bar(n)=>{
                if(value == 0xffff_ffff){
                    space.set_bar_size_read(n);
                    Ok(PciConfigAccessStatus::Done(0x0))
                }else if value == 0x0 {
                    Ok(PciConfigAccessStatus::Done(0x0))   
                }
                else{
                    
                    let a = space.get_bararr()[n];
                    let mut zone = this_zone();
                    let mut guard = zone.write();
                    guard.mmio_region_register(value , a.get_size() as usize, rng_mmio_handler, value);
                    drop(guard);
                    space.clear_bar_size_read(n);
                    
                    Ok(PciConfigAccessStatus::Done(0x0))
                }
            }
            _ => {
                Ok(PciConfigAccessStatus::Perform)
            }
        }
    }

    fn init_config_space(&self) -> PciConfigSpace {
        let mut space = PciConfigSpace::new();
        // let default_cspace = DEFAULT_CSPACE_U32;
        let default_cspace = COPY_CSPACE_U32;
        let mut offset = 0;
        for &value in &default_cspace {
            space.get_range_mut(offset, 2).copy_from_slice(&value.to_le_bytes());
            offset += 2;
        }
        
        // // Example: update vendor ID
        // space.set(EndpointField::ID, 0x12345678);
        
        space
    }
}

/// Static handler instance for standard virtual PCI devices
pub const HANDLER: VirtioRngHandler = VirtioRngHandler;

pub fn rng_mmio_handler(mmio: &mut MMIOAccess, _base: usize) -> HvResult {
    error!("i receive mmio!{:?}",mmio);
    // panic!("hhh!");
    Ok(())
}