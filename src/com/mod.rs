use std::ffi::c_void;
use std::ptr;
use windows_sys::core::{GUID, HRESULT};
use windows_sys::Win32::Foundation::S_OK;
use windows_sys::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_LOCAL_SERVER,
    COINIT_APARTMENTTHREADED, DISPPARAMS, EXCEPINFO,
};
use windows_sys::Win32::System::Variant::{
    VariantClear, VariantInit, VARIANT, VT_BOOL, VT_BSTR, VT_DISPATCH, VT_I4,
};

pub mod excel;
pub mod powerpoint;
pub mod word;

pub use excel::ExcelConverter;
pub use powerpoint::PowerPointConverter;
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

#[allow(non_snake_case)]
#[repr(C)]
pub struct IDispatchVtbl {
    pub QueryInterface: unsafe extern "system" fn(this: *mut c_void, riid: *const GUID, ppvObject: *mut *mut c_void) -> HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    pub GetTypeInfoCount: unsafe extern "system" fn(this: *mut c_void, pctinfo: *mut u32) -> HRESULT,
    pub GetTypeInfo: unsafe extern "system" fn(this: *mut c_void, iTInfo: u32, lcid: u32, ppTInfo: *mut *mut c_void) -> HRESULT,
    pub GetIDsOfNames: unsafe extern "system" fn(
        this: *mut c_void,
        riid: *const GUID,
        rgszNames: *const *const u16,
        cNames: u32,
        lcid: u32,
        rgDispId: *mut i32,
    ) -> HRESULT,
    pub Invoke: unsafe extern "system" fn(
        this: *mut c_void,
        dispIdMember: i32,
        riid: *const GUID,
        lcid: u32,
        wFlags: u16,
        pDispParams: *const DISPPARAMS,
        pVarResult: *mut VARIANT,
        pExcepInfo: *mut EXCEPINFO,
        puArgErr: *mut u32,
    ) -> HRESULT,
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct IDispatch {
    pub lpVtbl: *const IDispatchVtbl,
}

/// RAII Guard for COM library initialization on the current thread
pub struct ComApartment {
    _private: (),
}

impl ComApartment {
    pub fn new() -> Result<Self, String> {
        unsafe {
            // Office COM automation requires single-threaded apartment (STA)
            let hr = CoInitializeEx(ptr::null(), COINIT_APARTMENTTHREADED as u32);
            // S_OK or S_FALSE (already initialized) are both acceptable
            if hr < 0 && hr != -2147417850 { // RPC_E_CHANGED_MODE
                return Err(format!("CoInitializeEx failed with HRESULT 0x{:08X}", hr as u32));
            }
            Ok(ComApartment { _private: () })
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

/// Helper to convert a Rust string slice to a wide UTF-16 null-terminated vector
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Managed BSTR wrapper
pub struct ManagedBstr(pub *const u16);

impl ManagedBstr {
    pub fn from_str(s: &str) -> Self {
        unsafe {
            let wide = to_wide(s);
            // SysAllocStringLen takes length without null terminator
            let bstr = windows_sys::Win32::Foundation::SysAllocStringLen(
                wide.as_ptr(),
                (wide.len() - 1) as u32,
            );
            ManagedBstr(bstr)
        }
    }
}

impl Drop for ManagedBstr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::SysFreeString(self.0);
            }
        }
    }
}

/// Managed VARIANT wrapper
pub struct ManagedVariant(pub VARIANT);

impl ManagedVariant {
    pub fn empty() -> Self {
        unsafe {
            let mut v: VARIANT = std::mem::zeroed();
            VariantInit(&mut v);
            ManagedVariant(v)
        }
    }

    pub fn from_i4(val: i32) -> Self {
        unsafe {
            let mut v: VARIANT = std::mem::zeroed();
            VariantInit(&mut v);
            v.Anonymous.Anonymous.vt = VT_I4;
            v.Anonymous.Anonymous.Anonymous.lVal = val;
            ManagedVariant(v)
        }
    }

    pub fn from_bool(val: bool) -> Self {
        unsafe {
            let mut v: VARIANT = std::mem::zeroed();
            VariantInit(&mut v);
            v.Anonymous.Anonymous.vt = VT_BOOL;
            // VARIANT_TRUE is -1, VARIANT_FALSE is 0
            v.Anonymous.Anonymous.Anonymous.boolVal = if val { -1 } else { 0 };
            ManagedVariant(v)
        }
    }

    pub fn from_bstr(bstr: *const u16) -> Self {
        unsafe {
            let mut v: VARIANT = std::mem::zeroed();
            VariantInit(&mut v);
            v.Anonymous.Anonymous.vt = VT_BSTR;
            v.Anonymous.Anonymous.Anonymous.bstrVal = bstr as *mut u16;
            ManagedVariant(v)
        }
    }

    #[allow(dead_code)]
    pub fn from_dispatch(disp: *mut IDispatch) -> Self {
        unsafe {
            let mut v: VARIANT = std::mem::zeroed();
            VariantInit(&mut v);
            v.Anonymous.Anonymous.vt = VT_DISPATCH;
            v.Anonymous.Anonymous.Anonymous.pdispVal = disp as *mut c_void;
            ManagedVariant(v)
        }
    }
}

impl Drop for ManagedVariant {
    fn drop(&mut self) {
        unsafe {
            let _ = VariantClear(&mut self.0);
        }
    }
}

/// Managed IDispatch wrapper providing type-safe invoke helpers
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
