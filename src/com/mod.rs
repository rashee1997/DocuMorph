use std::ffi::c_void;
use std::ptr;
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::S_OK;
use windows_sys::Win32::System::Com::{
    CoCreateInstance, CLSCTX_LOCAL_SERVER, DISPPARAMS, EXCEPINFO,
};
use windows_sys::Win32::System::Variant::VARIANT;

pub mod excel;
pub mod powerpoint;
pub mod types;
pub mod word;

pub use excel::ExcelConverter;
pub use powerpoint::PowerPointConverter;
pub use types::*;
pub use word::WordConverter;

pub const DISPATCH_METHOD: u16 = 0x1;
pub const DISPATCH_PROPERTYGET: u16 = 0x2;
pub const DISPATCH_PROPERTYPUT: u16 = 0x4;
pub const DISPID_PROPERTYPUT: i32 = -3;

pub const IID_NULL: GUID = GUID {
    data1: 0,
    data2: 0,
    data3: 0,
    data4: [0; 8],
};

pub const IID_IDISPATCH: GUID = GUID {
    data1: 0x00020400,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

/// Managed IDispatch wrapper providing type-safe invoke helpers and deterministic RAII release
pub struct ComDispatch {
    raw: *mut IDispatch,
}

impl ComDispatch {
    pub fn create_instance(prog_id: &str) -> Result<Self, String> {
        unsafe {
            let wide_prog_id = to_wide(prog_id);
            let mut clsid: GUID = std::mem::zeroed();
            let hr = windows_sys::Win32::System::Com::CLSIDFromProgID(
                wide_prog_id.as_ptr(),
                &mut clsid,
            );
            if hr != S_OK {
                return Err(format!("ProgID '{}' is not registered on this system (HRESULT 0x{:08X})", prog_id, hr as u32));
            }

            let mut raw_ptr: *mut c_void = ptr::null_mut();
            let hr = CoCreateInstance(
                &clsid,
                ptr::null_mut(),
                CLSCTX_LOCAL_SERVER,
                &IID_IDISPATCH,
                &mut raw_ptr,
            );

            if hr != S_OK || raw_ptr.is_null() {
                return Err(format!("Failed to start '{}' via COM (HRESULT 0x{:08X})", prog_id, hr as u32));
            }

            Ok(ComDispatch {
                raw: raw_ptr as *mut IDispatch,
            })
        }
    }

    pub fn from_raw(raw: *mut IDispatch) -> Self {
        ComDispatch { raw }
    }

    #[allow(dead_code)]
    pub fn as_raw(&self) -> *mut IDispatch {
        self.raw
    }

    fn get_dispid(&self, name: &str) -> Result<i32, String> {
        unsafe {
            let wide_name = to_wide(name);
            let name_ptr = wide_name.as_ptr();
            let mut dispid = 0i32;

            let vtable = (*self.raw).lpVtbl;
            let hr = ((*vtable).GetIDsOfNames)(
                self.raw as *mut c_void,
                &IID_NULL,
                &name_ptr,
                1,
                0x0800, // LOCALE_SYSTEM_DEFAULT
                &mut dispid,
            );

            if hr != S_OK {
                return Err(format!("Member '{}' not found on COM object (HRESULT 0x{:08X})", name, hr as u32));
            }

            Ok(dispid)
        }
    }

    pub fn get_property(&self, name: &str) -> Result<ManagedVariant, String> {
        let dispid = self.get_dispid(name)?;
        unsafe {
            let params: DISPPARAMS = std::mem::zeroed();
            let mut result = ManagedVariant::empty();
            let mut excep: EXCEPINFO = std::mem::zeroed();
            let mut arg_err = 0u32;

            let vtable = (*self.raw).lpVtbl;
            let hr = ((*vtable).Invoke)(
                self.raw as *mut c_void,
                dispid,
                &IID_NULL,
                0x0800,
                DISPATCH_PROPERTYGET,
                &params,
                &mut result.0,
                &mut excep,
                &mut arg_err,
            );

            if hr != S_OK {
                return Err(format!("Get property '{}' failed (HRESULT 0x{:08X})", name, hr as u32));
            }

            Ok(result)
        }
    }

    pub fn get_dispatch_property(&self, name: &str) -> Result<ComDispatch, String> {
        self.get_property(name)?.to_dispatch()
    }

    pub fn set_property(&self, name: &str, mut value: ManagedVariant) -> Result<(), String> {
        let dispid = self.get_dispid(name)?;
        unsafe {
            let mut disp_named = DISPID_PROPERTYPUT;
            let mut params: DISPPARAMS = std::mem::zeroed();
            params.rgvarg = &mut value.0;
            params.rgdispidNamedArgs = &mut disp_named;
            params.cArgs = 1;
            params.cNamedArgs = 1;

            let mut excep: EXCEPINFO = std::mem::zeroed();
            let mut arg_err = 0u32;

            let vtable = (*self.raw).lpVtbl;
            let hr = ((*vtable).Invoke)(
                self.raw as *mut c_void,
                dispid,
                &IID_NULL,
                0x0800,
                DISPATCH_PROPERTYPUT,
                &params,
                ptr::null_mut(),
                &mut excep,
                &mut arg_err,
            );

            if hr != S_OK {
                return Err(format!("Set property '{}' failed (HRESULT 0x{:08X})", name, hr as u32));
            }

            Ok(())
        }
    }

    pub fn invoke_method(&self, name: &str, args: &mut [ManagedVariant]) -> Result<ManagedVariant, String> {
        let dispid = self.get_dispid(name)?;
        unsafe {
            // In COM IDispatch::Invoke, arguments must be passed in REVERSE order!
            let mut raw_args: Vec<VARIANT> = Vec::with_capacity(args.len());
            for arg in args.iter().rev() {
                raw_args.push(arg.0);
            }

            let mut params: DISPPARAMS = std::mem::zeroed();
            if !raw_args.is_empty() {
                params.rgvarg = raw_args.as_mut_ptr();
                params.cArgs = raw_args.len() as u32;
            }

            let mut result = ManagedVariant::empty();
            let mut excep: EXCEPINFO = std::mem::zeroed();
            let mut arg_err = 0u32;

            let vtable = (*self.raw).lpVtbl;
            let hr = ((*vtable).Invoke)(
                self.raw as *mut c_void,
                dispid,
                &IID_NULL,
                0x0800,
                DISPATCH_METHOD,
                &params,
                &mut result.0,
                &mut excep,
                &mut arg_err,
            );

            if hr != S_OK {
                let err_desc = if !excep.bstrDescription.is_null() {
                    let len = windows_sys::Win32::Foundation::SysStringLen(excep.bstrDescription) as usize;
                    let slice = std::slice::from_raw_parts(excep.bstrDescription, len);
                    String::from_utf16_lossy(slice)
                } else {
                    format!("HRESULT 0x{:08X}", hr as u32)
                };
                return Err(format!("Method '{}' failed: {}", name, err_desc));
            }

            Ok(result)
        }
    }
}

impl Drop for ComDispatch {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                let vtable = (*self.raw).lpVtbl;
                ((*vtable).Release)(self.raw as *mut c_void);
                self.raw = ptr::null_mut();
            }
        }
    }
}
