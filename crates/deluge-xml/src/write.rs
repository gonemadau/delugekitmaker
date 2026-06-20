use crate::{
    error::XmlResult,
    kit::{Drum, Flavor, Kit, OscSample},
};
use std::fmt::Write;

/// Serialize a [`Kit`] to a canonical Deluge kit XML string.
///
/// Uses a known-good template for all the synth-side params we don't model
/// (envelopes, LFOs, mod matrix, etc.) so the output is always a valid kit
/// that loads on official v4.x firmware and community Chopin.
pub fn write_kit(kit: &Kit, flavor: Flavor) -> XmlResult<String> {
    let mut out = String::with_capacity(8192);
    let fw = if kit.firmware_version.is_empty() {
        "4.1.4"
    } else {
        kit.firmware_version.as_str()
    };
    let earliest = if kit.earliest_compatible_firmware.is_empty() {
        "4.1.0-pre1"
    } else {
        kit.earliest_compatible_firmware.as_str()
    };

    writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
    writeln!(
        out,
        "<kit\n\tfirmwareVersion=\"{fw}\"\n\tearliestCompatibleFirmware=\"{earliest}\"\n\tlpfMode=\"24dB\"\n\tmodFXType=\"none\"\n\tmodFXCurrentParam=\"depth\"\n\tcurrentFilterType=\"lpf\">"
    )
    .unwrap();

    // top-level delay
    out.push_str(
        "\t<delay>\n\t\t<pingPong>1</pingPong>\n\t\t<analog>0</analog>\n\t\t<syncLevel>7</syncLevel>\n\t</delay>\n",
    );

    // kit-level defaultParams (mastered values for the kit as a whole)
    out.push_str(KIT_DEFAULT_PARAMS);

    // soundSources
    out.push_str("\t<soundSources>\n");
    for drum in &kit.drums {
        write_drum(&mut out, drum, flavor);
    }
    out.push_str("\t</soundSources>\n");

    out.push_str("\t<selectedDrumIndex>0</selectedDrumIndex>\n");
    out.push_str("</kit>\n");
    Ok(out)
}

fn write_drum(out: &mut String, drum: &Drum, _flavor: Flavor) {
    out.push_str("\t\t<sound>\n");
    writeln!(out, "\t\t\t<name>{}</name>", xml_escape(&drum.name)).unwrap();
    write_osc1(out, drum.osc1.as_ref());
    // osc2 disabled — always present in the schema but with type=sample and empty fileName
    out.push_str(
        "\t\t\t<osc2>\n\t\t\t\t<type>sample</type>\n\t\t\t\t<transpose>0</transpose>\n\t\t\t\t<cents>0</cents>\n\t\t\t\t<loopMode>0</loopMode>\n\t\t\t\t<reversed>0</reversed>\n\t\t\t\t<timeStretchEnable>0</timeStretchEnable>\n\t\t\t\t<timeStretchAmount>0</timeStretchAmount>\n\t\t\t\t<fileName></fileName>\n\t\t\t</osc2>\n",
    );

    // sound-level fixed elements
    out.push_str(
        "\t\t\t<polyphonic>auto</polyphonic>\n\t\t\t<voicePriority>1</voicePriority>\n\t\t\t<mode>subtractive</mode>\n",
    );
    out.push_str(
        "\t\t\t<lfo1>\n\t\t\t\t<type>sine</type>\n\t\t\t\t<syncLevel>0</syncLevel>\n\t\t\t</lfo1>\n\t\t\t<lfo2>\n\t\t\t\t<type>triangle</type>\n\t\t\t</lfo2>\n",
    );
    out.push_str("\t\t\t<unison>\n\t\t\t\t<num>1</num>\n\t\t\t\t<detune>8</detune>\n\t\t\t</unison>\n");
    out.push_str(
        "\t\t\t<delay>\n\t\t\t\t<pingPong>1</pingPong>\n\t\t\t\t<analog>0</analog>\n\t\t\t\t<syncLevel>7</syncLevel>\n\t\t\t</delay>\n",
    );
    out.push_str(
        "\t\t\t<compressor>\n\t\t\t\t<syncLevel>6</syncLevel>\n\t\t\t\t<attack>327244</attack>\n\t\t\t\t<release>936</release>\n\t\t\t</compressor>\n",
    );

    // per-drum defaultParams
    let volume = drum.volume_hex.as_deref().unwrap_or("0x4CCCCCA8");
    let pan = drum.pan_hex.as_deref().unwrap_or("0x00000000");
    write!(
        out,
        "{}",
        per_drum_default_params(volume, pan)
    )
    .unwrap();

    out.push_str("\t\t</sound>\n");
}

fn write_osc1(out: &mut String, osc: Option<&OscSample>) {
    out.push_str("\t\t\t<osc1>\n");
    out.push_str("\t\t\t\t<type>sample</type>\n");
    if let Some(o) = osc {
        writeln!(out, "\t\t\t\t<transpose>{}</transpose>", o.transpose).unwrap();
        writeln!(out, "\t\t\t\t<cents>{}</cents>", o.cents).unwrap();
        writeln!(out, "\t\t\t\t<loopMode>{}</loopMode>", o.loop_mode).unwrap();
        writeln!(
            out,
            "\t\t\t\t<reversed>{}</reversed>",
            if o.reversed { 1 } else { 0 }
        )
        .unwrap();
        out.push_str("\t\t\t\t<timeStretchEnable>0</timeStretchEnable>\n\t\t\t\t<timeStretchAmount>0</timeStretchAmount>\n");
        writeln!(out, "\t\t\t\t<fileName>{}</fileName>", xml_escape(&o.file_name)).unwrap();
        out.push_str("\t\t\t\t<zone>\n");
        writeln!(out, "\t\t\t\t\t<startSamplePos>{}</startSamplePos>", o.start_samples).unwrap();
        writeln!(out, "\t\t\t\t\t<endSamplePos>{}</endSamplePos>", o.end_samples).unwrap();
        out.push_str("\t\t\t\t</zone>\n");
    } else {
        out.push_str("\t\t\t\t<transpose>0</transpose>\n\t\t\t\t<cents>0</cents>\n\t\t\t\t<loopMode>0</loopMode>\n\t\t\t\t<reversed>0</reversed>\n\t\t\t\t<timeStretchEnable>0</timeStretchEnable>\n\t\t\t\t<timeStretchAmount>0</timeStretchAmount>\n\t\t\t\t<fileName></fileName>\n");
    }
    out.push_str("\t\t\t</osc1>\n");
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn per_drum_default_params(volume: &str, pan: &str) -> String {
    format!(
        "\t\t\t<defaultParams>\n\
\t\t\t\t<arpeggiatorGate>0x00000000</arpeggiatorGate>\n\
\t\t\t\t<portamento>0x80000000</portamento>\n\
\t\t\t\t<compressorShape>0xDC28F5B2</compressorShape>\n\
\t\t\t\t<oscAVolume>0x7FFFFFFF</oscAVolume>\n\
\t\t\t\t<oscAPulseWidth>0x00000000</oscAPulseWidth>\n\
\t\t\t\t<oscBVolume>0x80000000</oscBVolume>\n\
\t\t\t\t<oscBPulseWidth>0x00000000</oscBPulseWidth>\n\
\t\t\t\t<noiseVolume>0x80000000</noiseVolume>\n\
\t\t\t\t<volume>{volume}</volume>\n\
\t\t\t\t<pan>{pan}</pan>\n\
\t\t\t\t<lpfFrequency>0x7FFFFFFF</lpfFrequency>\n\
\t\t\t\t<lpfResonance>0x80000000</lpfResonance>\n\
\t\t\t\t<hpfFrequency>0x80000000</hpfFrequency>\n\
\t\t\t\t<hpfResonance>0x80000000</hpfResonance>\n\
\t\t\t\t<envelope1>\n\
\t\t\t\t\t<attack>0x80000000</attack>\n\
\t\t\t\t\t<decay>0xE6666654</decay>\n\
\t\t\t\t\t<sustain>0x7FFFFFD2</sustain>\n\
\t\t\t\t\t<release>0xE2000000</release>\n\
\t\t\t\t</envelope1>\n\
\t\t\t\t<envelope2>\n\
\t\t\t\t\t<attack>0xE6666654</attack>\n\
\t\t\t\t\t<decay>0xE6666654</decay>\n\
\t\t\t\t\t<sustain>0xFFFFFFE9</sustain>\n\
\t\t\t\t\t<release>0xE6666654</release>\n\
\t\t\t\t</envelope2>\n\
\t\t\t\t<lfo1Rate>0x1999997E</lfo1Rate>\n\
\t\t\t\t<lfo2Rate>0x00000000</lfo2Rate>\n\
\t\t\t\t<modulator1Amount>0x80000000</modulator1Amount>\n\
\t\t\t\t<modulator1Feedback>0x80000000</modulator1Feedback>\n\
\t\t\t\t<modulator2Amount>0x80000000</modulator2Amount>\n\
\t\t\t\t<modulator2Feedback>0x80000000</modulator2Feedback>\n\
\t\t\t\t<carrier1Feedback>0x80000000</carrier1Feedback>\n\
\t\t\t\t<carrier2Feedback>0x80000000</carrier2Feedback>\n\
\t\t\t\t<modFXRate>0x00000000</modFXRate>\n\
\t\t\t\t<modFXDepth>0x00000000</modFXDepth>\n\
\t\t\t\t<delay>\n\
\t\t\t\t\t<rate>0x00000000</rate>\n\
\t\t\t\t\t<feedback>0x80000000</feedback>\n\
\t\t\t\t</delay>\n\
\t\t\t\t<reverbAmount>0x80000000</reverbAmount>\n\
\t\t\t</defaultParams>\n"
    )
}

const KIT_DEFAULT_PARAMS: &str = "\t<defaultParams>\n\
\t\t<delay>\n\
\t\t\t<rate>0x00000000</rate>\n\
\t\t\t<feedback>0x80000000</feedback>\n\
\t\t</delay>\n\
\t\t<reverbAmount>0x80000000</reverbAmount>\n\
\t\t<volume>0x00000000</volume>\n\
\t\t<pan>0x00000000</pan>\n\
\t\t<lpf>\n\
\t\t\t<frequency>0x7FFFFFFF</frequency>\n\
\t\t\t<resonance>0x80000000</resonance>\n\
\t\t</lpf>\n\
\t\t<hpf>\n\
\t\t\t<frequency>0x80000000</frequency>\n\
\t\t\t<resonance>0x80000000</resonance>\n\
\t\t</hpf>\n\
\t\t<modFXDepth>0x00000000</modFXDepth>\n\
\t\t<modFXRate>0xE0000000</modFXRate>\n\
\t\t<stutterRate>0x00000000</stutterRate>\n\
\t\t<sampleRateReduction>0x80000000</sampleRateReduction>\n\
\t\t<bitCrush>0x80000000</bitCrush>\n\
\t\t<equalizer>\n\
\t\t\t<bass>0x00000000</bass>\n\
\t\t\t<treble>0x00000000</treble>\n\
\t\t\t<bassFrequency>0x00000000</bassFrequency>\n\
\t\t\t<trebleFrequency>0x00000000</trebleFrequency>\n\
\t\t</equalizer>\n\
\t\t<modFXOffset>0x00000000</modFXOffset>\n\
\t\t<modFXFeedback>0x80000000</modFXFeedback>\n\
\t</defaultParams>\n";
