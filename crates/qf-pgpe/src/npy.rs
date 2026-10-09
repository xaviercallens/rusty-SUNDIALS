//! Minimal reader/writer of numpy `.npy` files for complex and real arrays (versions 1-3, little endian, C order):
//! just enough to exchange wave functions with the Python programme and with published reproducibility packages.
use num_complex::Complex64;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Header facts of an `.npy` file.
#[derive(Debug, Clone)]
pub struct NpyHeader {
    pub descr: String,
    pub shape: Vec<usize>,
    pub fortran_order: bool,
    pub data_offset: usize,
}

pub fn parse_header(buf: &[u8]) -> Result<NpyHeader, String> {
    if buf.len() < 10 || &buf[..6] != b"\x93NUMPY" {
        return Err("not an .npy file".into());
    }
    let major = buf[6];
    let (hlen, start) = if major == 1 {
        (u16::from_le_bytes([buf[8], buf[9]]) as usize, 10)
    } else {
        (
            u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize,
            12,
        )
    };
    let h = std::str::from_utf8(&buf[start..start + hlen]).map_err(|e| e.to_string())?;
    let field = |key: &str| -> Option<&str> {
        let i = h.find(key)? + key.len();
        let rest = &h[i..];
        let j = rest.find(':')? + 1;
        Some(rest[j..].trim_start())
    };
    let descr = field("'descr'")
        .and_then(|s| s.strip_prefix('\''))
        .and_then(|s| s.split('\'').next())
        .ok_or("no descr")?
        .to_string();
    let fortran_order = field("'fortran_order'")
        .map(|s| s.starts_with("True"))
        .unwrap_or(false);
    let shape_s = field("'shape'")
        .and_then(|s| s.strip_prefix('('))
        .and_then(|s| s.split(')').next())
        .ok_or("no shape")?;
    let shape: Vec<usize> = shape_s
        .split(',')
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.trim().parse::<usize>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    Ok(NpyHeader {
        descr,
        shape,
        fortran_order,
        data_offset: start + hlen,
    })
}

/// Read a complex array (`<c8` complex64 or `<c16` complex128) in C order; returns `(shape, data)`.
pub fn read_complex(path: &Path) -> Result<(Vec<usize>, Vec<Complex64>), String> {
    let buf = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let h = parse_header(&buf)?;
    if h.fortran_order {
        return Err("fortran-order arrays are not supported".into());
    }
    let n: usize = h.shape.iter().product();
    let d = &buf[h.data_offset..];
    let f32at = |o: usize| f32::from_le_bytes(d[o..o + 4].try_into().unwrap()) as f64;
    let f64at = |o: usize| f64::from_le_bytes(d[o..o + 8].try_into().unwrap());
    match h.descr.as_str() {
        "<c8" if d.len() >= 8 * n => Ok((
            h.shape,
            (0..n)
                .map(|i| Complex64::new(f32at(8 * i), f32at(8 * i + 4)))
                .collect(),
        )),
        "<c16" if d.len() >= 16 * n => Ok((
            h.shape,
            (0..n)
                .map(|i| Complex64::new(f64at(16 * i), f64at(16 * i + 8)))
                .collect(),
        )),
        other => Err(format!(
            "unsupported dtype {other} (need <c8 or <c16) or short data"
        )),
    }
}

/// Write a complex128 array (`<c16`, C order, version 1 header).
pub fn write_complex(path: &Path, shape: &[usize], data: &[Complex64]) -> std::io::Result<()> {
    let shp = if shape.len() == 1 {
        format!("({},)", shape[0])
    } else {
        format!(
            "({})",
            shape
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let mut header = format!("{{'descr': '<c16', 'fortran_order': False, 'shape': {shp}, }}");
    let pad = (64 - (10 + header.len() + 1) % 64) % 64;
    header.push_str(&" ".repeat(pad));
    header.push('\n');
    let mut f = fs::File::create(path)?;
    f.write_all(b"\x93NUMPY\x01\x00")?;
    f.write_all(&(header.len() as u16).to_le_bytes())?;
    f.write_all(header.as_bytes())?;
    for v in data {
        f.write_all(&v.re.to_le_bytes())?;
        f.write_all(&v.im.to_le_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex128_roundtrip_and_header() {
        let p = std::env::temp_dir().join("qf_pgpe_npy_roundtrip.npy");
        let data: Vec<Complex64> = (0..12)
            .map(|i| Complex64::new(i as f64 * 0.5, -(i as f64)))
            .collect();
        write_complex(&p, &[3, 4], &data).unwrap();
        let (shape, back) = read_complex(&p).unwrap();
        assert_eq!(shape, vec![3, 4]);
        assert_eq!(back, data);
        let h = parse_header(&std::fs::read(&p).unwrap()).unwrap();
        assert_eq!((h.descr.as_str(), h.data_offset % 64), ("<c16", 0));
    }
}
