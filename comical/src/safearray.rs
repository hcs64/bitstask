use std::default::Default;
use std::marker::PhantomData;
use std::ptr::null_mut;
use std::slice;

use winapi::shared::ntdef::ULONG;
use winapi::shared::winerror::HRESULT;
use winapi::shared::wtypes::{BSTR, VARTYPE, VT_BSTR};
use winapi::um::oaidl::SAFEARRAY;
use winapi::um::oleauto::{SafeArrayAccessData, SafeArrayCreateVector, SafeArrayUnaccessData};

use bstr::BStr;
use com::check_nonnull;
use handle::check_hresult;

// TODO: PR for winapi-rs
extern "system" {
    fn SafeArrayDestroy(psa: *mut SAFEARRAY) -> HRESULT;
}

pub struct SafeArray<T: Copy + 'static> {
    raw: *mut SAFEARRAY,
    phantom: PhantomData<T>,
}

impl<T: Copy + 'static> SafeArray<T> {
    pub fn get(&mut self) -> *mut SAFEARRAY {
        self.raw
    }
}

impl<T: Copy + 'static> Drop for SafeArray<T> {
    fn drop(&mut self) {
        unsafe { SafeArrayDestroy(self.raw) };
    }
}

impl SafeArray<BSTR> {
    pub fn try_new(elements: ULONG) -> Result<Self, String> {
        Ok(SafeArray {
            raw: check_nonnull("SafeArrayCreateVector", unsafe {
                SafeArrayCreateVector(VT_BSTR as VARTYPE, 0, elements)
            })?,
            phantom: Default::default(),
        })
    }

    pub fn try_from(vec: Vec<BStr>) -> Result<Self, String> {
        let mut array = Self::try_new(vec.len() as ULONG)?;
        {
            let access = SafeArrayAccess::new(&mut array)?;
            let v = unsafe { slice::from_raw_parts_mut(access.get_data(), vec.len()) };
            for (elt, mut src) in v.iter_mut().zip(vec.into_iter()) {
                *elt = src.take()
            }
        }
        Ok(array)
    }
}

pub struct SafeArrayAccess<'a, T: Copy + 'static> {
    array: Option<&'a mut SafeArray<T>>,
    data: *mut T,
}

impl<'a, T: Copy + 'static> SafeArrayAccess<'a, T> {
    unsafe fn getter(array: *mut SAFEARRAY) -> Result<*mut T, String> {
        let mut data = null_mut();
        check_hresult(
            "SafeArrayAccessData",
            SafeArrayAccessData(array, &mut data as *mut *mut T as *mut *mut _),
        )?;
        Ok(data)
    }

    pub fn new(array: &'a mut SafeArray<T>) -> Result<Self, String> {
        Ok(SafeArrayAccess {
            data: unsafe { Self::getter(array.get()) }?,
            array: Some(array),
        })
    }

    pub unsafe fn from_raw(array: *mut SAFEARRAY) -> Result<Self, String> {
        Ok(SafeArrayAccess {
            array: None,
            data: Self::getter(array)?,
        })
    }

    pub fn get_data(self) -> *mut T {
        self.data
    }
}

impl<'a, T: Copy + 'static> Drop for SafeArrayAccess<'a, T> {
    fn drop(&mut self) {
        if let Some(ref mut array) = self.array {
            unsafe { SafeArrayUnaccessData(array.get()) };
        }
    }
}
