use super::super::{Animation, Effect, Gradient, Palette};

fn rgb(c: [u8; 3]) -> String {
    format!(
        "float3({:.9},{:.9},{:.9})",
        c[0] as f64 / 255.0,
        c[1] as f64 / 255.0,
        c[2] as f64 / 255.0
    )
}
fn layer(name: &str, primary: [u8; 3], e: Effect) -> String {
    let angle = (e.angle_degrees as f64).to_radians();
    let scale = 100.0 / e.spread_percent as f64;
    let coord = match e.gradient {
        Gradient::Solid => "0.0".into(),
        Gradient::Linear => format!(
            "0.5+dot(uv-0.5,float2({:.9},{:.9}))*{scale:.9}",
            angle.cos(),
            angle.sin()
        ),
        Gradient::Radial => format!("length(uv-0.5)*2.0*{scale:.9}"),
    };
    let motion = match e.animation {
        Animation::None | Animation::Rainbow => String::new(),
        Animation::ColorFade if e.gradient == Gradient::Solid => {
            "t=0.5-0.5*cos(phase*6.283185307);".into()
        }
        Animation::ColorFade => "t=lerp(t,1.0-t,0.5-0.5*cos(phase*6.283185307));".into(),
        Animation::Flow => "t=0.5-0.5*cos((coord*0.5-phase)*6.283185307);".into(),
    };
    let rainbow = if e.animation == Animation::Rainbow {
        "c=saturate(abs(frac(phase+coord*0.25+float3(0,0.666666667,0.333333333))*6-3)-1)*max(c.r,max(c.g,c.b));"
    } else {
        ""
    };
    format!("float3 {name}(float2 uv,float time){{float coord={coord};float t=saturate(coord);float phase=time*{speed:.9};{motion}float3 c=lerp({a},{b},t);{rainbow}return c;}}\n",speed=1000.0/e.period_ms as f64,a=rgb(primary),b=rgb(e.secondary))
}
pub(super) fn source(p: Palette) -> String {
    let mut s = String::from("cbuffer CB0:register(b0){float4 FogColor_GlobalForceFieldTime:packoffset(c19);float2 invViewportWH:packoffset(c50);};\n");
    s.push_str(&layer("background", p.background, p.background_effect));
    s.push_str(&layer("surface", p.surface, p.surface_effect));
    s.push_str(&layer("accent", p.accent, p.accent_effect));
    s.push_str("float4 main(float4 color:COLOR0,float4 position:SV_Position):SV_Target{float3 c=color.rgb;float hi=max(c.r,max(c.g,c.b));float lo=min(c.r,min(c.g,c.b));float2 uv=position.xy*invViewportWH;float time=FogColor_GlobalForceFieldTime.w;if(hi-lo<0.04 && hi<0.40)c=lerp(background(uv,time),surface(uv,time),hi/0.40);if(color.b>color.r*1.55 && color.b>color.g*1.30 && color.b>0.025)c=accent(uv,time);return float4(c,color.a);}");
    s
}
