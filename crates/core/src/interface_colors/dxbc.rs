//! Local DXBC transformation. The original texture, clipping, alpha, gradient,
//! and output-signature instructions are preserved; RGB mapping is appended.
use super::Palette;
use anyhow::{ensure, Context, Result};

fn word(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4).context("Truncated DXBC")?.try_into()?,
    ))
}
fn checksum(data: &[u8]) -> Result<[u8; 16]> {
    let source = data.get(20..).context("Truncated DXBC checksum input")?;
    let bits = u32::try_from(source.len())?
        .checked_mul(8)
        .context("DXBC too large")?;
    let full = source.len() / 64 * 64;
    let tail = &source[full..];
    let mut blocks = source[..full].to_vec();
    if tail.len() >= 56 {
        blocks.extend(tail);
        blocks.push(128);
        blocks.resize(full + 64, 0);
        blocks.extend(bits.to_le_bytes());
        blocks.resize(full + 124, 0);
        blocks.extend(((bits >> 2) | 1).to_le_bytes());
    } else {
        blocks.extend(bits.to_le_bytes());
        blocks.extend(tail);
        blocks.push(128);
        blocks.resize(full + 60, 0);
        blocks.extend(((bits >> 2) | 1).to_le_bytes());
    }
    let mut state = [0x67452301u32, 0xefcdab89, 0x98badcfe, 0x10325476];
    let shifts = [
        [7, 12, 17, 22],
        [5, 9, 14, 20],
        [4, 11, 16, 23],
        [6, 10, 15, 21],
    ];
    for block in blocks.chunks_exact(64) {
        let [mut a, mut b, mut c, mut d] = state;
        for i in 0..64usize {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), 7 * i % 16),
            };
            let k = ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32;
            let z = a
                .wrapping_add(f)
                .wrapping_add(k)
                .wrapping_add(word(block, g * 4)?);
            (a, d, c, b) = (
                d,
                c,
                b,
                b.wrapping_add(z.rotate_left(shifts[i / 16][i % 4])),
            );
        }
        for (s, v) in state.iter_mut().zip([a, b, c, d]) {
            *s = s.wrapping_add(v);
        }
    }
    let mut result = [0; 16];
    for (part, value) in result.chunks_exact_mut(4).zip(state) {
        part.copy_from_slice(&value.to_le_bytes());
    }
    Ok(result)
}
type Chunk = ([u8; 4], Vec<u8>);
fn chunks(data: &[u8]) -> Result<Vec<Chunk>> {
    ensure!(
        data.get(..4) == Some(b"DXBC") && word(data, 24)? as usize == data.len(),
        "Invalid DXBC container"
    );
    ensure!(
        data.get(4..20) == Some(checksum(data)?.as_slice()),
        "DXBC checksum failed"
    );
    let count = word(data, 28)? as usize;
    ensure!(
        (1..=32).contains(&count) && data.len() >= 32 + 4 * count,
        "Invalid DXBC chunk count"
    );
    let mut result = Vec::new();
    for i in 0..count {
        let p = word(data, 32 + i * 4)? as usize;
        ensure!(p >= 32 + count * 4, "Invalid DXBC chunk offset");
        let len = word(data, p + 4)? as usize;
        let end = p
            .checked_add(8)
            .and_then(|p| p.checked_add(len))
            .context("DXBC chunk overflow")?;
        let name = data
            .get(p..p + 4)
            .context("Missing DXBC chunk name")?
            .try_into()?;
        result.push((
            name,
            data.get(p + 8..end)
                .context("Truncated DXBC chunk")?
                .to_vec(),
        ));
    }
    Ok(result)
}
fn assemble(chunks: &[Chunk]) -> Result<Vec<u8>> {
    let mut data = b"DXBC".to_vec();
    data.resize(20, 0);
    for value in [1, 0, chunks.len() as u32] {
        data.extend(value.to_le_bytes());
    }
    data.resize(32 + chunks.len() * 4, 0);
    for (i, (name, body)) in chunks.iter().enumerate() {
        let offset = u32::try_from(data.len())?;
        data[32 + i * 4..36 + i * 4].copy_from_slice(&offset.to_le_bytes());
        data.extend(name);
        data.extend(u32::try_from(body.len())?.to_le_bytes());
        data.extend(body);
    }
    let length = u32::try_from(data.len())?;
    data[24..28].copy_from_slice(&length.to_le_bytes());
    let digest = checksum(&data)?;
    data[4..20].copy_from_slice(&digest);
    Ok(data)
}
fn program(chunks: &[Chunk]) -> Result<(usize, Vec<u32>)> {
    let matches: Vec<_> = chunks
        .iter()
        .enumerate()
        .filter(|(_, c)| [b"SHDR", b"SHEX"].contains(&&c.0))
        .collect();
    ensure!(matches.len() == 1, "Expected one shader program");
    let (i, (_, data)) = matches[0];
    ensure!(data.len() % 4 == 0, "Invalid shader word length");
    let words: Vec<_> = data
        .chunks_exact(4)
        .map(|p| u32::from_le_bytes(p.try_into().expect("four bytes")))
        .collect();
    ensure!(
        words.len() >= 2 && words[0] == 0x50 && words[1] as usize == words.len(),
        "Expected a complete ps_5_0 program"
    );
    Ok((i, words))
}
fn instructions(words: &[u32]) -> Result<Vec<Vec<u32>>> {
    let mut p = 2;
    let mut result = Vec::new();
    while p < words.len() {
        let mut n = ((words[p] >> 24) & 127) as usize;
        if words[p] & 2047 == 53 {
            n = *words.get(p + 1).context("Invalid custom data")? as usize;
        }
        ensure!(
            n > 0 && p.checked_add(n).is_some_and(|end| end <= words.len()),
            "Truncated shader instruction"
        );
        result.push(words[p..p + n].to_vec());
        p += n;
    }
    Ok(result)
}
fn remap_operand(words: &mut [u32], p: usize, base: u32) -> Result<usize> {
    let token = *words.get(p).context("Missing mapper operand")?;
    let mut index_pos = p + 1;
    let mut extended = token;
    while extended >> 31 != 0 {
        extended = *words
            .get(index_pos)
            .context("Truncated extended mapper operand")?;
        index_pos += 1;
    }
    let kind = (token >> 12) & 255;
    if kind == 4 {
        let count = match token & 3 {
            1 => 1,
            2 => 4,
            _ => anyhow::bail!("Invalid mapper literal"),
        };
        ensure!(index_pos + count <= words.len(), "Truncated mapper literal");
        return Ok(index_pos + count);
    }
    ensure!(
        (token >> 20) & 3 == 1 && (token >> 22) & 7 == 0,
        "Expected an immediate-indexed mapper register"
    );
    let index = *words
        .get(index_pos)
        .context("Missing mapper register index")?;
    match kind {
        0 => {
            ensure!(index < 16, "Mapper temporary out of range");
            words[index_pos] = index + base + 1;
        }
        1 => {
            ensure!(index == 0, "Unexpected mapper input");
            words[p] &= !(255 << 12);
            words[index_pos] = base;
        }
        2 => ensure!(index == 0, "Unexpected mapper output"),
        _ => anyhow::bail!("Unexpected mapper register kind"),
    }
    Ok(index_pos + 1)
}
pub(super) fn patch(original: &[u8], mapper: &[u8]) -> Result<Vec<u8>> {
    let mut cc = chunks(original)?;
    let (index, words) = program(&cc)?;
    let mut ins = instructions(&words)?;
    let (_, mapper_words) = program(&chunks(mapper)?)?;
    let mapper_ins = instructions(&mapper_words)?;
    let counts: Vec<_> = ins
        .iter()
        .filter(|x| x[0] & 2047 == 104)
        .map(|x| x.get(1).copied())
        .collect();
    let mapper_counts: Vec<_> = mapper_ins
        .iter()
        .filter(|x| x[0] & 2047 == 104)
        .map(|x| x.get(1).copied())
        .collect();
    ensure!(
        counts.len() == 1 && mapper_counts.len() == 1,
        "Invalid temporary declarations"
    );
    let base = counts[0].context("Missing temporary count")?;
    let extra = mapper_counts[0].context("Missing mapper temporary count")?;
    ensure!(base < 128 && extra < 16, "Too many shader temporaries");
    for instruction in &mut ins {
        if instruction[0] & 2047 == 104 {
            instruction[1] = base + 1 + extra;
        }
    }
    let mut changed = 0;
    for i in 0..ins.len() {
        if ![54, 55, 56, 75].contains(&(ins[i][0] & 2047))
            || ins[i].len() < 3
            || (ins[i][1] >> 12) & 255 != 2
        {
            continue;
        }
        if i + 1 < ins.len() - 1 && ins[i + 1] == [0x0100003e] {
            ensure!(
                ins[i][0] & 2047 == 54
                    && ins[i].get(3) == Some(&0x4002)
                    && ins[i].get(4..) == Some(&[0, 0, 0, 0]),
                "Unexpected early shader return"
            );
            continue;
        }
        ensure!(
            ins[i][2] == 0 && (ins[i][1] >> 20) & 3 == 1,
            "Unexpected shader output"
        );
        ins[i][1] &= !(255 << 12);
        ins[i][2] = base;
        changed += 1;
    }
    ensure!(
        (1..=2).contains(&changed) && ins.last().is_some_and(|x| x == &[0x0100003e]),
        "Unexpected GUI output path"
    );
    ins.pop();
    for mut instruction in mapper_ins {
        if [98, 101, 104, 106].contains(&(instruction[0] & 2047)) {
            continue;
        }
        ensure!(
            instruction[0] >> 31 == 0,
            "Unexpected extended mapper opcode"
        );
        let mut p = 1;
        while p < instruction.len() {
            p = remap_operand(&mut instruction, p, base)?;
        }
        ensure!(p == instruction.len(), "Invalid mapper instruction");
        ins.push(instruction);
    }
    let mut program = words[..2].to_vec();
    program.extend(ins.into_iter().flatten());
    program[1] = program.len() as u32;
    cc[index].1 = program.iter().flat_map(|x| x.to_le_bytes()).collect();
    let result = assemble(&cc)?;
    validate(&result)?;
    Ok(result)
}
pub(super) fn compile(palette: Palette) -> Result<Vec<u8>> {
    let rgb = |a: [u8; 3]| {
        format!(
            "float3({:.9},{:.9},{:.9})",
            a[0] as f64 / 255.0,
            a[1] as f64 / 255.0,
            a[2] as f64 / 255.0
        )
    };
    let source = format!(
        r#"
float4 main(float4 color:COLOR0):SV_Target {{
 float3 c=color.rgb;
 float hi=max(c.r,max(c.g,c.b)); float lo=min(c.r,min(c.g,c.b));
 bool neutral=(hi-lo<0.04 && hi<0.40);
 bool blue=(c.b>c.r*1.55 && c.b>c.g*1.30 && c.b>0.025);
 if(neutral) c={bg}+(hi/0.40)*({surface}-{bg});
 if(blue) c={accent};
 return float4(c,color.a);
}}
"#,
        bg = rgb(palette.background),
        surface = rgb(palette.surface),
        accent = rgb(palette.accent)
    );
    native::compile(&source)
}
fn validate(data: &[u8]) -> Result<()> {
    native::validate(data)
}

#[cfg(not(windows))]
mod native {
    use super::*;
    pub fn compile(_: &str) -> Result<Vec<u8>> {
        anyhow::bail!("GUI shader colors require Windows Direct3D 11")
    }
    pub fn validate(_: &[u8]) -> Result<()> {
        anyhow::bail!("GUI shader validation requires Windows")
    }
}
#[cfg(windows)]
mod native {
    use super::*;
    use std::ffi::{c_char, c_void};
    type Ptr = *mut c_void;
    #[link(name = "d3dcompiler")]
    extern "system" {
        fn D3DCompile(
            data: *const c_void,
            size: usize,
            name: *const c_char,
            defines: *const c_void,
            include: *const c_void,
            entry: *const c_char,
            target: *const c_char,
            flags: u32,
            flags2: u32,
            code: *mut Ptr,
            errors: *mut Ptr,
        ) -> i32;
    }
    #[link(name = "d3d11")]
    extern "system" {
        fn D3D11CreateDevice(
            adapter: Ptr,
            driver: u32,
            software: Ptr,
            flags: u32,
            levels: *const u32,
            count: u32,
            sdk: u32,
            device: *mut Ptr,
            level: *mut u32,
            context: *mut Ptr,
        ) -> i32;
    }
    struct Com(Ptr);
    impl Com {
        unsafe fn method(&self, index: usize) -> Ptr {
            *(*(self.0 as *const *const Ptr)).add(index)
        }
        unsafe fn blob(&self) -> Vec<u8> {
            let ptr: unsafe extern "system" fn(Ptr) -> *const u8 =
                std::mem::transmute(self.method(3));
            let size: unsafe extern "system" fn(Ptr) -> usize = std::mem::transmute(self.method(4));
            std::slice::from_raw_parts(ptr(self.0), size(self.0)).to_vec()
        }
    }
    impl Drop for Com {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let release: unsafe extern "system" fn(Ptr) -> u32 =
                        std::mem::transmute(self.method(2));
                    release(self.0);
                }
            }
        }
    }
    pub fn compile(source: &str) -> Result<Vec<u8>> {
        let mut code = Com(std::ptr::null_mut());
        let mut errors = Com(std::ptr::null_mut());
        let hr = unsafe {
            D3DCompile(
                source.as_ptr().cast(),
                source.len(),
                c"SOLstrap GUI palette".as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                c"main".as_ptr(),
                c"ps_5_0".as_ptr(),
                1 << 15,
                0,
                &mut code.0,
                &mut errors.0,
            )
        };
        let message = if errors.0.is_null() {
            String::new()
        } else {
            String::from_utf8_lossy(&unsafe { errors.blob() }).into_owned()
        };
        ensure!(
            hr >= 0 && !code.0.is_null(),
            "Cannot compile GUI palette: {hr:#x} {message}"
        );
        let data = unsafe { code.blob() };
        chunks(&data)?;
        validate(&data)?;
        Ok(data)
    }
    pub fn validate(data: &[u8]) -> Result<()> {
        let mut device = Com(std::ptr::null_mut());
        let mut context = Com(std::ptr::null_mut());
        let mut level = 0;
        let hr = unsafe {
            D3D11CreateDevice(
                std::ptr::null_mut(),
                5,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                0,
                7,
                &mut device.0,
                &mut level,
                &mut context.0,
            )
        };
        ensure!(
            hr >= 0 && !device.0.is_null(),
            "Direct3D software validation unavailable: {hr:#x}"
        );
        let create: unsafe extern "system" fn(Ptr, *const c_void, usize, Ptr, *mut Ptr) -> i32 =
            unsafe { std::mem::transmute(device.method(15)) };
        let mut shader = Com(std::ptr::null_mut());
        let hr = unsafe {
            create(
                device.0,
                data.as_ptr().cast(),
                data.len(),
                std::ptr::null_mut(),
                &mut shader.0,
            )
        };
        ensure!(
            hr >= 0 && !shader.0.is_null(),
            "Direct3D rejected GUI palette: {hr:#x}"
        );
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn compiled_checksum_roundtrip_and_rejection() {
        for source in [
            "float4 main(float4 c:COLOR0):SV_Target{return c;}",
            "float4 main(float4 c:COLOR0):SV_Target{return c*c+0.12345;}",
        ] {
            let data = native::compile(source).unwrap();
            let rebuilt = assemble(&chunks(&data).unwrap()).unwrap();
            validate(&rebuilt).unwrap();
            let mut damaged = data.clone();
            *damaged.last_mut().unwrap() ^= 1;
            assert!(chunks(&damaged).is_err());
            assert!(chunks(&data[..data.len() - 1]).is_err());
        }
    }
    #[test]
    fn palette_patch_validates_with_independent_original_shader() {
        let original =
            native::compile("float4 main(float4 c:COLOR0):SV_Target{return saturate(c*c)*c;}")
                .unwrap();
        let mapper = compile(Palette::default()).unwrap();
        let patched = patch(&original, &mapper).unwrap();
        validate(&patched).unwrap();
        let before = chunks(&original).unwrap();
        let after = chunks(&patched).unwrap();
        assert_eq!(before.len(), after.len());
        for (a, b) in before.iter().zip(&after) {
            if ![b"SHDR", b"SHEX"].contains(&&a.0) {
                assert_eq!(a, b);
            }
        }
    }
}
