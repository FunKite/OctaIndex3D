//! Verification example: Confirm Index64 and Hilbert64 are both working

use octaindex3d::hilbert::Hilbert64;
use octaindex3d::ids::{FrameId, Index64};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OctaIndex3D Verification: Index64 & Hilbert64 ===\n");

    // ========== INDEX64 VERIFICATION ==========
    println!("📍 INDEX64 Verification");
    println!("───────────────────────");

    let frame: FrameId = 2;
    let tier: u8 = 1;
    let lod: u8 = 7;
    let (x, y, z) = (512u16, 768u16, 1024u16);

    let index64 = Index64::new(frame, tier, lod, x, y, z)?;
    println!(
        "✓ Created Index64: frame={}, tier={}, lod={}, coords=({}, {}, {})",
        frame, tier, lod, x, y, z
    );

    // Verify extraction
    let extracted_frame = index64.frame_id();
    let extracted_coords = index64.decode_coords();
    println!(
        "✓ Extracted: frame={}, coords=({}, {}, {})",
        extracted_frame, extracted_coords.0, extracted_coords.1, extracted_coords.2
    );

    assert_eq!(extracted_frame, frame, "Frame mismatch!");
    assert_eq!(extracted_coords, (x, y, z), "Coords mismatch!");
    println!("✓ Index64 roundtrip verification PASSED\n");

    // ========== HILBERT64 VERIFICATION ==========
    println!("🌀 HILBERT64 Verification");
    println!("───────────────────────");

    let h_frame: FrameId = 1;
    let h_tier: u8 = 2;
    let h_lod: u8 = 8;
    let (hx, hy, hz) = (1024u16, 2048u16, 4096u16);

    let hilbert64 = Hilbert64::new(h_frame, h_tier, h_lod, hx, hy, hz)?;
    println!(
        "✓ Created Hilbert64: frame={}, tier={}, lod={}, coords=({}, {}, {})",
        h_frame, h_tier, h_lod, hx, hy, hz
    );

    // Verify extraction
    let h_extracted_frame = hilbert64.frame_id();
    let h_extracted_coords = hilbert64.decode();
    println!(
        "✓ Extracted: frame={}, coords=({}, {}, {})",
        h_extracted_frame, h_extracted_coords.0, h_extracted_coords.1, h_extracted_coords.2
    );

    assert_eq!(h_extracted_frame, h_frame, "Hilbert frame mismatch!");
    assert_eq!(h_extracted_coords, (hx, hy, hz), "Hilbert coords mismatch!");
    println!("✓ Hilbert64 roundtrip verification PASSED\n");

    // ========== INDEX64 ↔ HILBERT64 CONVERSION ==========
    println!("🔄 Interoperability Test");
    println!("───────────────────────");

    let idx = Index64::new(3, 0, 6, 256, 512, 768)?;
    println!("✓ Created Index64: coords=({}, {}, {})", 256, 512, 768);

    let hilb: Hilbert64 = idx.try_into()?;
    println!("✓ Converted to Hilbert64");

    let idx_back: Index64 = hilb.into();
    println!("✓ Converted back to Index64");

    let coords_original = idx.decode_coords();
    let coords_roundtrip = idx_back.decode_coords();
    println!(
        "✓ Original coords:     ({}, {}, {})",
        coords_original.0, coords_original.1, coords_original.2
    );
    println!(
        "✓ Roundtrip coords:    ({}, {}, {})",
        coords_roundtrip.0, coords_roundtrip.1, coords_roundtrip.2
    );

    assert_eq!(
        coords_original, coords_roundtrip,
        "Roundtrip conversion failed!"
    );
    println!("✓ Index64 ↔ Hilbert64 conversion PASSED\n");

    // ========== BATCH OPERATIONS ==========
    println!("📦 Batch Operations Test");
    println!("───────────────────────");

    let coords = vec![
        (100u16, 200u16, 300u16),
        (200u16, 400u16, 600u16),
        (300u16, 600u16, 900u16),
    ];

    let batch = Hilbert64::encode_batch(&coords, 0, 0, 5)?;
    println!("✓ Batch encoded {} Hilbert64 keys", batch.len());

    for (i, h) in batch.iter().enumerate() {
        let decoded = h.decode();
        assert_eq!(decoded, coords[i], "Batch {} roundtrip failed!", i);
    }
    println!("✓ All batch keys decoded correctly\n");

    // ========== SUMMARY ==========
    println!("═════════════════════════════════════════");
    println!("✅ ALL VERIFICATIONS PASSED!");
    println!("═════════════════════════════════════════");
    println!("\nBoth Index64 and Hilbert64 are running correctly:");
    println!("  • Index64:     64-bit Morton tile keys ✓");
    println!("  • Hilbert64:   64-bit Hilbert curve keys ✓");
    println!("  • Conversion:  Index64 ↔ Hilbert64 ✓");
    println!("  • Batch ops:   Batch encoding/decoding ✓");

    Ok(())
}
