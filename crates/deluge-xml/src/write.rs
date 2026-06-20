use crate::{
    error::XmlResult,
    kit::{Drum, Flavor, Kit, OscSample, XML_TO_PAD},
};
use std::fmt::Write;

/// Serialize a [`Kit`] to a canonical Deluge kit XML string.
///
/// Output matches the v3-style format that's known to load on real Deluge
/// hardware: no `firmwareVersion` / `earliestCompatibleFirmware` gate on
/// `<kit>`, zones use `<startMilliseconds>` / `<endMilliseconds>` child
/// elements, and every sound carries the full `<defaultParams>`, `<midiKnobs>`,
/// and `<modKnobs>` blocks the Deluge expects.
///
/// `end_ms` of 0 on a sample means "no duration known" — the writer leaves a
/// zero zone there, but `deluge_fs::save_kit` is responsible for filling that
/// in from the WAV header before calling this function.
pub fn write_kit(kit: &Kit, _flavor: Flavor) -> XmlResult<String> {
    let mut out = String::with_capacity(16384);

    writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
    out.push_str("<kit>\n");
    out.push_str("\t<lpfMode>24dB</lpfMode>\n");
    out.push_str("\t<modFXType>flanger</modFXType>\n");
    out.push_str("\t<modFXCurrentParam>feedback</modFXCurrentParam>\n");
    out.push_str("\t<currentFilterType>lpf</currentFilterType>\n");
    out.push_str(KIT_DEFAULT_PARAMS);

    out.push_str("\t<soundSources>\n");
    // Walk the 16 UI pads in XML order (bottom row first, top row last) so the
    // Deluge displays the kit in the same orientation as our UI.
    for xml_slot in 0..16 {
        let pad = XML_TO_PAD[xml_slot];
        if let Some(drum) = kit.drums.get(pad) {
            if drum.osc1.as_ref().map(|o| !o.file_name.is_empty()).unwrap_or(false) {
                write_drum(&mut out, drum);
            }
        }
    }
    out.push_str("\t</soundSources>\n");
    out.push_str("\t<selectedDrumIndex>0</selectedDrumIndex>\n");
    out.push_str("</kit>\n");
    Ok(out)
}

fn write_drum(out: &mut String, drum: &Drum) {
    out.push_str("\t\t<sound>\n");
    writeln!(out, "\t\t\t<name>{}</name>", xml_escape(&drum.name)).unwrap();
    write_osc1(out, drum.osc1.as_ref().unwrap());
    out.push_str(OSC2_BLOCK);
    out.push_str(SOUND_FIXED_TAIL);
    write_sound_default_params(out, drum);
    out.push_str(MIDI_AND_MOD_KNOBS);
    out.push_str("\t\t</sound>\n");
}

fn write_osc1(out: &mut String, osc: &OscSample) {
    out.push_str("\t\t\t<osc1>\n");
    out.push_str("\t\t\t\t<type>sample</type>\n");
    writeln!(out, "\t\t\t\t<transpose>{}</transpose>", osc.transpose).unwrap();
    writeln!(out, "\t\t\t\t<cents>{}</cents>", osc.cents).unwrap();
    writeln!(out, "\t\t\t\t<loopMode>{}</loopMode>", osc.loop_mode).unwrap();
    writeln!(out, "\t\t\t\t<reversed>{}</reversed>", if osc.reversed { 1 } else { 0 }).unwrap();
    out.push_str("\t\t\t\t<timeStretchEnable>0</timeStretchEnable>\n");
    out.push_str("\t\t\t\t<timeStretchAmount>0</timeStretchAmount>\n");
    writeln!(out, "\t\t\t\t<fileName>{}</fileName>", xml_escape(&osc.file_name)).unwrap();
    out.push_str("\t\t\t\t<zone>\n");
    writeln!(out, "\t\t\t\t\t<startMilliseconds>{}</startMilliseconds>", osc.start_ms).unwrap();
    // end_ms 0 here would produce a silent zone — by convention the FS layer
    // populates it from the WAV. If a caller passes us 0, we still emit it so
    // tests can detect the bug.
    writeln!(out, "\t\t\t\t\t<endMilliseconds>{}</endMilliseconds>", osc.end_ms).unwrap();
    out.push_str("\t\t\t\t</zone>\n");
    out.push_str("\t\t\t</osc1>\n");
}

fn write_sound_default_params(out: &mut String, drum: &Drum) {
    let volume = drum.volume_hex.as_deref().unwrap_or("0x7FFFFFFF");
    let pan = drum.pan_hex.as_deref().unwrap_or("0x00000000");
    writeln!(out, "\t\t\t<defaultParams>").unwrap();
    out.push_str("\t\t\t\t<arpeggiatorGate>0x00000000</arpeggiatorGate>\n");
    out.push_str("\t\t\t\t<portamento>0x80000000</portamento>\n");
    out.push_str("\t\t\t\t<compressorShape>0xDC28F5B2</compressorShape>\n");
    out.push_str("\t\t\t\t<oscAVolume>0x7FFFFFD2</oscAVolume>\n");
    out.push_str("\t\t\t\t<oscAPulseWidth>0x00000000</oscAPulseWidth>\n");
    out.push_str("\t\t\t\t<oscBVolume>0x80000000</oscBVolume>\n");
    out.push_str("\t\t\t\t<oscBPulseWidth>0x00000000</oscBPulseWidth>\n");
    out.push_str("\t\t\t\t<noiseVolume>0x80000000</noiseVolume>\n");
    writeln!(out, "\t\t\t\t<volume>{}</volume>", volume).unwrap();
    writeln!(out, "\t\t\t\t<pan>{}</pan>", pan).unwrap();
    out.push_str("\t\t\t\t<lpfFrequency>0x7FFFFFD2</lpfFrequency>\n");
    out.push_str("\t\t\t\t<lpfResonance>0xFFFFFFE9</lpfResonance>\n");
    out.push_str("\t\t\t\t<hpfFrequency>0x80000000</hpfFrequency>\n");
    out.push_str("\t\t\t\t<hpfResonance>0x80000000</hpfResonance>\n");
    out.push_str(SOUND_ENVELOPES_AND_MODS);
    out.push_str("\t\t\t</defaultParams>\n");
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const KIT_DEFAULT_PARAMS: &str = "\t<defaultParams>\n\
\t\t<delay>\n\
\t\t\t<rate>0x00000000</rate>\n\
\t\t\t<feedback>0x80000000</feedback>\n\
\t\t</delay>\n\
\t\t<reverbAmount>0x80000000</reverbAmount>\n\
\t\t<volume>0x3504F334</volume>\n\
\t\t<pan>0x00000000</pan>\n\
\t\t<lpf>\n\
\t\t\t<frequency>0x7FFFFFFF</frequency>\n\
\t\t\t<resonance>0x00000000</resonance>\n\
\t\t</lpf>\n\
\t\t<hpf>\n\
\t\t\t<frequency>0x80000000</frequency>\n\
\t\t\t<resonance>0xC0000000</resonance>\n\
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

const OSC2_BLOCK: &str = "\t\t\t<osc2>\n\
\t\t\t\t<type>square</type>\n\
\t\t\t\t<transpose>0</transpose>\n\
\t\t\t\t<cents>0</cents>\n\
\t\t\t\t<retrigPhase>0</retrigPhase>\n\
\t\t\t</osc2>\n";

const SOUND_FIXED_TAIL: &str = "\t\t\t<polyphonic>0</polyphonic>\n\
\t\t\t<clippingAmount>0</clippingAmount>\n\
\t\t\t<voicePriority>1</voicePriority>\n\
\t\t\t<lfo1>\n\
\t\t\t\t<type>sine</type>\n\
\t\t\t\t<syncLevel>0</syncLevel>\n\
\t\t\t</lfo1>\n\
\t\t\t<lfo2>\n\
\t\t\t\t<type>sine</type>\n\
\t\t\t</lfo2>\n\
\t\t\t<mode>subtractive</mode>\n\
\t\t\t<unison>\n\
\t\t\t\t<num>1</num>\n\
\t\t\t\t<detune>16</detune>\n\
\t\t\t</unison>\n\
\t\t\t<compressor>\n\
\t\t\t\t<syncLevel>6</syncLevel>\n\
\t\t\t\t<attack>168220</attack>\n\
\t\t\t\t<release>936</release>\n\
\t\t\t</compressor>\n\
\t\t\t<delay>\n\
\t\t\t\t<pingPong>1</pingPong>\n\
\t\t\t\t<analog>0</analog>\n\
\t\t\t\t<syncLevel>7</syncLevel>\n\
\t\t\t</delay>\n\
\t\t\t<lpfMode>12dB</lpfMode>\n\
\t\t\t<modFXType>none</modFXType>\n";

const SOUND_ENVELOPES_AND_MODS: &str = "\t\t\t\t<envelope1>\n\
\t\t\t\t\t<attack>0x80000000</attack>\n\
\t\t\t\t\t<decay>0xE6666654</decay>\n\
\t\t\t\t\t<sustain>0x7FFFFFD2</sustain>\n\
\t\t\t\t\t<release>0x80000000</release>\n\
\t\t\t\t</envelope1>\n\
\t\t\t\t<envelope2>\n\
\t\t\t\t\t<attack>0xE6666654</attack>\n\
\t\t\t\t\t<decay>0xE6666654</decay>\n\
\t\t\t\t\t<sustain>0xFFFFFFE9</sustain>\n\
\t\t\t\t\t<release>0xE6666654</release>\n\
\t\t\t\t</envelope2>\n\
\t\t\t\t<lfo1Rate>0x00000000</lfo1Rate>\n\
\t\t\t\t<lfo2Rate>0x00000000</lfo2Rate>\n\
\t\t\t\t<modulator1Amount>0x80000000</modulator1Amount>\n\
\t\t\t\t<modulator1Feedback>0x80000000</modulator1Feedback>\n\
\t\t\t\t<modulator2Amount>0x80000000</modulator2Amount>\n\
\t\t\t\t<modulator2Feedback>0x80000000</modulator2Feedback>\n\
\t\t\t\t<carrier1Feedback>0x80000000</carrier1Feedback>\n\
\t\t\t\t<carrier2Feedback>0x80000000</carrier2Feedback>\n\
\t\t\t\t<modFXRate>0x2135ACA7</modFXRate>\n\
\t\t\t\t<modFXDepth>0x00460027</modFXDepth>\n\
\t\t\t\t<delayRate>0x00000000</delayRate>\n\
\t\t\t\t<delayFeedback>0x80000000</delayFeedback>\n\
\t\t\t\t<reverbAmount>0x80000000</reverbAmount>\n\
\t\t\t\t<arpeggiatorRate>0x00000000</arpeggiatorRate>\n\
\t\t\t\t<patchCables>\n\
\t\t\t\t\t<patchCable>\n\
\t\t\t\t\t\t<source>velocity</source>\n\
\t\t\t\t\t\t<destination>pitch</destination>\n\
\t\t\t\t\t\t<amount>0x3FFFFFE8</amount>\n\
\t\t\t\t\t</patchCable>\n\
\t\t\t\t</patchCables>\n\
\t\t\t\t<stutterRate>0x00000000</stutterRate>\n\
\t\t\t\t<sampleRateReduction>0x80000000</sampleRateReduction>\n\
\t\t\t\t<bitCrush>0x80000000</bitCrush>\n\
\t\t\t\t<equalizer>\n\
\t\t\t\t\t<bass>0x00000000</bass>\n\
\t\t\t\t\t<treble>0x00000000</treble>\n\
\t\t\t\t\t<bassFrequency>0x00000000</bassFrequency>\n\
\t\t\t\t\t<trebleFrequency>0x00000000</trebleFrequency>\n\
\t\t\t\t</equalizer>\n\
\t\t\t\t<modFXOffset>0xFFC00000</modFXOffset>\n\
\t\t\t\t<modFXFeedback>0xFFFFFFAA</modFXFeedback>\n";

const MIDI_AND_MOD_KNOBS: &str = "\t\t\t<midiKnobs>\n\
\t\t\t</midiKnobs>\n\
\t\t\t<modKnobs>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>pan</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>volumePostFX</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>lpfResonance</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>lpfFrequency</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>env1Release</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>env1Attack</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>delayFeedback</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>delayRate</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>reverbAmount</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>volumePostReverbSend</controlsParam>\n\
\t\t\t\t\t<patchAmountFromSource>compressor</patchAmountFromSource>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>pitch</controlsParam>\n\
\t\t\t\t\t<patchAmountFromSource>lfo1</patchAmountFromSource>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>lfo1Rate</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>pitch</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>stutterRate</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>bitcrushAmount</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t\t<modKnob>\n\
\t\t\t\t\t<controlsParam>sampleRateReduction</controlsParam>\n\
\t\t\t\t</modKnob>\n\
\t\t\t</modKnobs>\n";
