use std::ffi::c_void;
use std::ptr;
use windows_sys::core::{GUID, HRESULT};
use windows_sys::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, DISPPARAMS, EXCEPINFO,
};
use windows_sys::Win32::System::Variant::{
    VariantClear, VariantInit, VARIANT, VT_BOOL, VT_BSTR, VT_DISPATCH, VT_EMPTY, VT_I4,
};

use super::ComDispatch;

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

    pub fn to_dispatch(mut self) -> Result<ComDispatch, String> {
        unsafe {
            if self.0.Anonymous.Anonymous.vt != VT_DISPATCH {
                return Err(format!("Variant is not VT_DISPATCH (vt={})", self.0.Anonymous.Anonymous.vt));
            }
            let ptr = self.0.Anonymous.Anonymous.Anonymous.pdispVal as *mut IDispatch;
            if ptr.is_null() {
                return Err("Dispatch pointer is null".to_string());
            }
            // Clear VT so VariantClear won't release it on drop, transferring ownership to ComDispatch
            self.0.Anonymous.Anonymous.vt = VT_EMPTY;
            Ok(ComDispatch::from_raw(ptr))
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
