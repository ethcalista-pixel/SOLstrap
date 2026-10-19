//! Merge only the generated mapper's global buffer and pixel-position input.
//! Original resource bindings, variables, and signature registers stay intact.
use super::{word, Chunk};
use anyhow::{ensure, Context, Result};

fn put(data: &mut [u8], at: usize, value: u32) -> Result<()> {
    data.get_mut(at..at + 4)
        .context("Truncated reflection field")?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}
fn name(data: &[u8], offset: u32) -> Result<&[u8]> {
    let s = data
        .get(offset as usize..)
        .context("Invalid reflection string")?;
    let n = s
        .iter()
        .position(|b| *b == 0)
        .context("Unterminated reflection string")?;
    Ok(&s[..n])
}
fn table(
    data: &[u8],
    count_at: usize,
    offset_at: usize,
    stride: usize,
    limit: usize,
) -> Result<Vec<Vec<u8>>> {
    let count = word(data, count_at)? as usize;
    let offset = word(data, offset_at)? as usize;
    ensure!(count <= limit, "Unexpected reflection table size");
    Ok(data
        .get(offset..offset + count * stride)
        .context("Truncated reflection table")?
        .chunks_exact(stride)
        .map(|b| b.to_vec())
        .collect())
}
fn buffer(original: &mut Vec<u8>, mapper: &[u8], required: u32) -> Result<()> {
    let mut cb = table(original, 0, 4, 24, 32)?;
    let mut bindings = table(original, 8, 12, 32, 64)?;
    if let Some(binding) = bindings
        .iter()
        .find(|b| word(b, 4).ok() == Some(0) && word(b, 20).ok() == Some(0))
    {
        let buffer_name = name(original, word(binding, 0)?)?;
        let existing = cb
            .iter()
            .find(|b| name(original, word(b, 0).unwrap_or(u32::MAX)).ok() == Some(buffer_name))
            .context("Missing original global buffer reflection")?;
        ensure!(
            word(existing, 12)? >= required * 16,
            "Original global buffer is too small"
        );
        return Ok(());
    }
    let mcb = table(mapper, 0, 4, 24, 1)?;
    let mb = table(mapper, 8, 12, 32, 1)?;
    ensure!(
        mcb.len() == 1
            && mb.len() == 1
            && word(&mb[0], 4)? == 0
            && word(&mb[0], 20)? == 0
            && word(&mb[0], 24)? == 1,
        "Unexpected generated buffer binding"
    );
    ensure!(
        word(&mcb[0], 12)? >= required * 16,
        "Generated buffer is too small"
    );
    let base = u32::try_from(original.len())?;
    let mut relocated = mapper.to_vec();
    let desc = word(mapper, 4)? as usize;
    let vars = word(mapper, desc + 4)? as usize;
    let var_at = word(mapper, desc + 8)? as usize;
    ensure!(vars <= 2 && vars > 0, "Unexpected generated variables");
    let rebase = |data: &mut [u8], at: usize| -> Result<()> {
        let old = word(data, at)?;
        if old != 0 {
            ensure!(
                (old as usize) < mapper.len(),
                "Invalid generated reflection pointer"
            );
            put(
                data,
                at,
                old.checked_add(base)
                    .context("Reflection offset overflow")?,
            )?;
        }
        Ok(())
    };
    let mut types = std::collections::BTreeSet::new();
    for i in 0..vars {
        let v = var_at + i * 40;
        ensure!(
            [8, 16].contains(&word(mapper, v + 8)?) && word(mapper, v + 20)? == 0,
            "Unexpected generated variable shape"
        );
        let t = word(mapper, v + 16)? as usize;
        types.insert(t);
        rebase(&mut relocated, v)?;
        rebase(&mut relocated, v + 16)?;
    }
    for t in types {
        ensure!(
            mapper.get(t..t + 6) == Some(&[1, 0, 3, 0, 1, 0])
                && mapper.get(t + 6).is_some_and(|v| [2, 4].contains(v))
                && mapper
                    .get(t + 7..t + 12)
                    .is_some_and(|b| b.iter().all(|v| *v == 0))
                && mapper
                    .get(t + 12..t + 32)
                    .is_some_and(|b| b.iter().all(|v| *v == 0)),
            "Unexpected generated float vector type"
        );
        rebase(&mut relocated, t + 32)?;
    }
    rebase(&mut relocated, desc)?;
    rebase(&mut relocated, desc + 8)?;
    let bind = word(mapper, 12)? as usize;
    rebase(&mut relocated, bind)?;
    cb.push(relocated[desc..desc + 24].to_vec());
    bindings.push(relocated[bind..bind + 32].to_vec());
    original.extend(relocated);
    put(original, 0, cb.len() as u32)?;
    let offset = original.len() as u32;
    put(original, 4, offset)?;
    original.extend(cb.into_iter().flatten());
    put(original, 8, bindings.len() as u32)?;
    let offset = original.len() as u32;
    put(original, 12, offset)?;
    original.extend(bindings.into_iter().flatten());
    Ok(())
}
pub(super) fn merge(
    cc: &mut [Chunk],
    mapper: &[Chunk],
    ins: &mut Vec<Vec<u32>>,
    mapper_ins: &[Vec<u32>],
) -> Result<Option<u32>> {
    let cb = mapper_ins.iter().find(|x| x[0] & 2047 == 89);
    let pos = mapper_ins.iter().find(|x| x[0] & 2047 == 100);
    if cb.is_none() && pos.is_none() {
        return Ok(None);
    }
    if let Some(decl) = cb {
        ensure!(
            decl.len() == 4 && decl[2] == 0 && decl[3] <= 51,
            "Unexpected mapper globals declaration"
        );
        if let Some(existing) = ins
            .iter_mut()
            .find(|x| x[0] & 2047 == 89 && x.get(2) == Some(&0))
        {
            ensure!(
                existing.len() == 4,
                "Unexpected original globals declaration"
            );
            existing[3] = existing[3].max(decl[3]);
        } else {
            ins.insert(0, decl.clone());
        }
        let r = cc
            .iter_mut()
            .find(|c| &c.0 == b"RDEF")
            .context("Missing original reflection")?;
        let m = mapper
            .iter()
            .find(|c| &c.0 == b"RDEF")
            .context("Missing mapper reflection")?;
        buffer(&mut r.1, &m.1, decl[3])?;
    }
    let Some(decl) = pos else { return Ok(None) };
    ensure!(
        decl.len() == 4 && decl[2] == 1 && decl[3] == 1,
        "Unexpected mapper position declaration"
    );
    let mask = ((decl[1] >> 4) & 15) as u8;
    let signature = &mut cc
        .iter_mut()
        .find(|c| &c.0 == b"ISGN")
        .context("Missing input signature")?
        .1;
    let count = word(signature, 0)? as usize;
    ensure!(count <= 32, "Unexpected input signature size");
    let mut register = None;
    for i in 0..count {
        let at = 8 + i * 24;
        if word(signature, at + 8)? == 1 {
            ensure!(
                register.is_none()
                    && name(signature, word(signature, at)?)?.eq_ignore_ascii_case(b"SV_Position"),
                "Unexpected pixel position signature"
            );
            ensure!(
                signature.get(at + 20).is_some_and(|m| m & mask == mask),
                "Pixel position components unavailable"
            );
            register = Some(word(signature, at + 16)?);
            *signature
                .get_mut(at + 21)
                .context("Truncated signature mask")? |= mask;
        }
    }
    let register = register.context("Original shader has no pixel position")?;
    if let Some(existing) = ins
        .iter_mut()
        .find(|x| [98, 100].contains(&(x[0] & 2047)) && x.get(2) == Some(&register))
    {
        existing[1] |= (mask as u32) << 4;
    } else {
        let mut d = decl.clone();
        d[2] = register;
        ins.insert(0, d);
    }
    Ok(Some(register))
}
