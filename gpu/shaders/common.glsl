// Intetrigence Vulkan Compute S2 Beam Search — shared definitions & S2 rules.
// 100% original implementation matching crates/core and crates/bot/src/eval.rs.

const uint MAX_WIDTH = 2048u;
const uint MAX_CHILDREN = 262144u;
const uint MAX_ROOTS = 256u;
const uint BOARD_HEIGHT = 40u;
const uint BOARD_WIDTH = 10u;
const uint FULL_ROW = 0x3FFu;
const uint GARBAGE_CAP = 8u;
const uint OPENER_PIECES = 14u;
const uint COMBO_CAP = 4u;
const uint DANGER_THRESHOLD = 12u;
const uint SURVIVAL_PROJECTED = 21u;
const int  DANGER_BASE = 15;
const uint B2B_CHARGE_AT = 4u;
const uint B2B_CHARGE_BASE = 3u;
const uint PC_GARBAGE = 5u;

// Piece kinds (matching crates/core/src/piece.rs):
// 0=I, 1=O, 2=T, 3=S, 4=Z, 5=J, 6=L
// Rotations: 0=Zero, 1=R, 2=Two, 3=L
// SpinType: 0=None, 1=Mini, 2=Full

struct GpuNode {
    uint rows[20];      // 40 rows packed 2 per uint (lo10 = row 2i, hi10 at <<16 = row 2i+1)
    uint gmask_lo;      // garbage row bitmask rows 0..31
    uint gmask_hi;      // garbage row bitmask rows 32..39
    uint meta0;         // [0..7]=next, [8..15]=hold(255=none), [16..23]=b2b, [24..31]=combo
    uint meta1;         // [0..7]=incoming, [8..15]=arrived, [16..23]=root_idx, [24..31]=flags
    uint sent_total;
    uint received_total;
    uint pieces_placed;
    float reward;
    float value;
    uint key_lo;
    uint key_hi;
    uint pad0;
};

struct LockForecast {
    uint eligible;
    uint hypothetical;
    uint extra;
    float multiplier;
};

struct EvalWeights {
    float max_height;
    float height_sq;
    float danger_height;
    float quad_ready;
    float defense;
    float risk_base;
    float risk_span;
    float risk_floor;
    float opener_clear;
    float combo_state;
    float combo_potential;
    float race;
    float holes;
    float hole_cover;
    float hole_rows;
    float bumpiness;
    float bumpiness_sq;
    float row_transitions;
    float well_depth;
    float extra_wells;
    float t_slot0;
    float t_slot1;
    float t_slot2;
    float danger_projected;
    float stack_exposure;
    float garbage_rows;
    float attack;
    float root_sent;
    // clear[spin * 5 + lines]: 15 floats
    float clear_w[15];
    float perfect_clear;
    float b2b_break;
    float b2b_stock;
    float margin_stock;
    float wasted_t;
    float surge_charge;
    float surge_exponent;
    float surge_hold_penalty;
    float tsd_bonus;
    float mini_penalty;
    float combo;
    float opener_attack;
    float garbage_clear;
    float survival_flag; // > 0.5 means true
    float survival_line;
    float incoming;
    float discount;
    // v3 S2 features (12 floats -> total 72 floats = 288 bytes)
    float garbage_cover;
    float center_height;
    float col_transitions;
    float t_slot_open_ratio;
    float cancel_bonus;
    float hold_t_bonus;
    float hold_i_bonus;
    float height_mult_scale;
    float surge_cashout_bonus;
    float well_center_penalty;
    float downstack_urgency;
    float b2b_keep_mult;
};

struct SearchParams {
    uint seq[24];
    uint seq_len;
    uint depth;
    uint width;
    uint initial_incoming;
    uint projection_mode; // 0=Stop, 1=Straight, 2=SplitBurst
    uint root_count;
    uint pad0;
    uint pad1;
    LockForecast locks[24];
    EvalWeights w;
};

struct RuntimeState {
    uint child_count;
    uint beam_count;
    uint level;
    uint overflow;
    // Indirect dispatch args at offset 16 bytes (VkDispatchIndirectCommand)
    uint dispatch_x;
    uint dispatch_y;
    uint dispatch_z;
    uint pad0;
};

uint float_to_sortable(float f) {
    uint u = floatBitsToUint(f);
    return ((u & 0x80000000u) != 0u) ? ~u : (u | 0x80000000u);
}

float sortable_to_float(uint s) {
    uint u = ((s & 0x80000000u) != 0u) ? (s & 0x7FFFFFFFu) : ~s;
    return uintBitsToFloat(u);
}

uint get_row(const in uint rows[20], uint y) {
    return (rows[y >> 1u] >> ((y & 1u) * 16u)) & FULL_ROW;
}

void set_row(inout uint rows[20], uint y, uint val) {
    uint idx = y >> 1u;
    uint sh = (y & 1u) * 16u;
    rows[idx] = (rows[idx] & ~(0xFFFFu << sh)) | ((val & FULL_ROW) << sh);
}

bool get_gbit(uint glo, uint ghi, uint y) {
    return (y < 32u) ? (((glo >> y) & 1u) != 0u) : (((ghi >> (y - 32u)) & 1u) != 0u);
}

void set_gbit(inout uint glo, inout uint ghi, uint y, bool v) {
    if (y < 32u) {
        if (v) glo |= (1u << y); else glo &= ~(1u << y);
    } else {
        uint bit = 1u << (y - 32u);
        if (v) ghi |= bit; else ghi &= ~bit;
    }
}

uint board_height(const in uint rows[20]) {
    for (int i = 19; i >= 0; i--) {
        uint w = rows[i];
        if (w != 0u) {
            return ((w >> 16u) != 0u) ? uint(i * 2 + 2) : uint(i * 2 + 1);
        }
    }
    return 0u;
}

// Piece local cells: packed as (cx | (cy << 4)) for 4 cells per (kind, rot).
// Derived identically to crates/core/src/piece.rs::PieceKind::cells.
ivec2 piece_cell(uint kind, uint rot, uint i) {
    ivec2 c = ivec2(0);
    int maxb = (kind == 0u) ? 3 : ((kind == 1u) ? 1 : 2);
    if (kind == 0u) { // I
        c = ivec2(int(i), 2);
    } else if (kind == 1u) { // O
        const ivec2 o[4] = ivec2[4](ivec2(0,0), ivec2(1,0), ivec2(0,1), ivec2(1,1));
        c = o[i];
    } else if (kind == 2u) { // T
        const ivec2 t[4] = ivec2[4](ivec2(1,2), ivec2(0,1), ivec2(1,1), ivec2(2,1));
        c = t[i];
    } else if (kind == 3u) { // S
        const ivec2 s[4] = ivec2[4](ivec2(1,2), ivec2(2,2), ivec2(0,1), ivec2(1,1));
        c = s[i];
    } else if (kind == 4u) { // Z
        const ivec2 z[4] = ivec2[4](ivec2(0,2), ivec2(1,2), ivec2(1,1), ivec2(2,1));
        c = z[i];
    } else if (kind == 5u) { // J
        const ivec2 j[4] = ivec2[4](ivec2(0,2), ivec2(0,1), ivec2(1,1), ivec2(2,1));
        c = j[i];
    } else { // L
        const ivec2 l[4] = ivec2[4](ivec2(2,2), ivec2(0,1), ivec2(1,1), ivec2(2,1));
        c = l[i];
    }
    for (uint s = 0u; s < rot; s++) {
        c = ivec2(c.y, maxb - c.x);
    }
    return c;
}

ivec2 spawn_pos(uint kind) {
    if (kind == 0u) return ivec2(3, 19);
    if (kind == 1u) return ivec2(4, 21);
    return ivec2(3, 20);
}

// SRS+ kick tables matching crates/core/src/kick.rs (KickOrder::SrsPlus).
// 90-degree transition index: 0->R=0, R->0=1, R->2=2, 2->R=3, 2->L=4, L->2=5, L->0=6, 0->L=7
int kick90_index(uint from, uint to) {
    if (from == 0u && to == 1u) return 0;
    if (from == 1u && to == 0u) return 1;
    if (from == 1u && to == 2u) return 2;
    if (from == 2u && to == 1u) return 3;
    if (from == 2u && to == 3u) return 4;
    if (from == 3u && to == 2u) return 5;
    if (from == 3u && to == 0u) return 6;
    if (from == 0u && to == 3u) return 7;
    return -1;
}

int kick180_index(uint from, uint to) {
    if (from == 0u && to == 2u) return 0;
    if (from == 2u && to == 0u) return 1;
    if (from == 1u && to == 3u) return 2;
    if (from == 3u && to == 1u) return 3;
    return -1;
}

const ivec2 JLSTZ_K90[40] = ivec2[40](
    ivec2(0,0), ivec2(-1,0), ivec2(-1,1), ivec2(0,-2), ivec2(-1,-2), // 0->R
    ivec2(0,0), ivec2(1,0),  ivec2(1,-1), ivec2(0,2),  ivec2(1,2),   // R->0
    ivec2(0,0), ivec2(1,0),  ivec2(1,-1), ivec2(0,2),  ivec2(1,2),   // R->2
    ivec2(0,0), ivec2(-1,0), ivec2(-1,1), ivec2(0,-2), ivec2(-1,-2), // 2->R
    ivec2(0,0), ivec2(1,0),  ivec2(1,1),  ivec2(0,-2), ivec2(1,-2),  // 2->L
    ivec2(0,0), ivec2(-1,0), ivec2(-1,-1),ivec2(0,2),  ivec2(-1,2),  // L->2
    ivec2(0,0), ivec2(-1,0), ivec2(-1,-1),ivec2(0,2),  ivec2(-1,2),  // L->0
    ivec2(0,0), ivec2(1,0),  ivec2(1,1),  ivec2(0,-2), ivec2(1,-2)   // 0->L
);

const ivec2 I_K90_SRSP[40] = ivec2[40](
    ivec2(0,0), ivec2(1,0),  ivec2(-2,0), ivec2(-2,-1), ivec2(1,2),  // 0->R
    ivec2(0,0), ivec2(-1,0), ivec2(2,0),  ivec2(-1,-2), ivec2(2,1),  // R->0
    ivec2(0,0), ivec2(-1,0), ivec2(2,0),  ivec2(-1,2),  ivec2(2,-1), // R->2
    ivec2(0,0), ivec2(-2,0), ivec2(1,0),  ivec2(-2,1),  ivec2(1,-2), // 2->R
    ivec2(0,0), ivec2(2,0),  ivec2(-1,0), ivec2(2,1),   ivec2(-1,-2),// 2->L
    ivec2(0,0), ivec2(1,0),  ivec2(-2,0), ivec2(1,2),   ivec2(-2,-1),// L->2
    ivec2(0,0), ivec2(1,0),  ivec2(-2,0), ivec2(1,-2),  ivec2(-2,1), // L->0
    ivec2(0,0), ivec2(-1,0), ivec2(2,0),  ivec2(2,-1),  ivec2(-1,2)  // 0->L
);

const ivec2 JLSTZ_K180[24] = ivec2[24](
    ivec2(0,0), ivec2(0,1),  ivec2(1,1),  ivec2(-1,1), ivec2(1,0),  ivec2(-1,0), // 0->2
    ivec2(0,0), ivec2(0,-1), ivec2(-1,-1),ivec2(1,-1), ivec2(-1,0), ivec2(1,0),  // 2->0
    ivec2(0,0), ivec2(1,0),  ivec2(1,2),  ivec2(1,1),  ivec2(0,2),  ivec2(0,1),  // R->L
    ivec2(0,0), ivec2(-1,0), ivec2(-1,2), ivec2(-1,1), ivec2(0,2),  ivec2(0,1)   // L->R
);

const ivec2 I_K180[8] = ivec2[8](
    ivec2(0,0), ivec2(0,1),
    ivec2(0,0), ivec2(0,-1),
    ivec2(0,0), ivec2(1,0),
    ivec2(0,0), ivec2(-1,0)
);

bool is_fin_or_tst(uint from, uint to, ivec2 k) {
    if ((from == 0u || from == 2u) && to == 3u) return (k.x == 1 && k.y == -2);
    if ((from == 0u || from == 2u) && to == 1u) return (k.x == -1 && k.y == -2);
    return false;
}

bool can_place_board(const in uint rows[20], uint kind, uint rot, int x, int y) {
    for (uint i = 0u; i < 4u; i++) {
        ivec2 c = piece_cell(kind, rot, i);
        int bx = x + c.x;
        int by = y + c.y;
        if (bx < 0 || bx >= 10 || by < 0 || by >= 40) return false;
        if ((get_row(rows, uint(by)) & (1u << uint(bx))) != 0u) return false;
    }
    return true;
}

bool is_immobile_board(const in uint rows[20], uint kind, uint rot, int x, int y) {
    return !can_place_board(rows, kind, rot, x - 1, y)
        && !can_place_board(rows, kind, rot, x + 1, y)
        && !can_place_board(rows, kind, rot, x, y - 1)
        && !can_place_board(rows, kind, rot, x, y + 1);
}

bool corner_filled(const in uint rows[20], int x, int y) {
    if (x < 0 || x >= 10 || y < 0) return true;
    if (y >= 40) return false;
    return (get_row(rows, uint(y)) & (1u << uint(x))) != 0u;
}

uint classify_spin(const in uint rows[20], uint kind, uint rot, int x, int y, bool spin_entry, bool fin_or_tst) {
    if (!spin_entry) return 0u;
    if (kind == 2u) { // T
        const ivec2 CORNERS[4] = ivec2[4](ivec2(-1,-1), ivec2(1,-1), ivec2(1,1), ivec2(-1,1));
        uint corners = 0u;
        uint front = 0u;
        for (uint idx = 0u; idx < 4u; idx++) {
            if (corner_filled(rows, x + 1 + CORNERS[idx].x, y + 1 + CORNERS[idx].y)) {
                corners++;
                bool is_front = false;
                if (idx == 0u) is_front = (rot == 2u || rot == 3u);
                else if (idx == 1u) is_front = (rot == 1u || rot == 2u);
                else if (idx == 2u) is_front = (rot == 0u || rot == 1u);
                else is_front = (rot == 3u || rot == 0u);
                if (is_front) front++;
            }
        }
        if (corners >= 3u) {
            return (front == 2u || fin_or_tst) ? 2u : 1u;
        }
    }
    return is_immobile_board(rows, kind, rot, x, y) ? 1u : 0u;
}

uint surge_charge_from_streak(uint b2b) {
    uint disp = (b2b > 0u) ? (b2b - 1u) : 0u;
    if (disp >= B2B_CHARGE_AT) {
        return disp - B2B_CHARGE_AT + B2B_CHARGE_BASE + 1u;
    }
    return 0u;
}

uint base_attack_val(uint spin, uint lines) {
    const uint tbl[15] = uint[15](
        0u, 0u, 1u, 2u, 4u,
        0u, 0u, 1u, 2u, 10u,
        0u, 2u, 4u, 6u, 10u
    );
    return tbl[spin * 5u + lines];
}

// Best nub-down T-slot: returns ivec4(bx, y, lines, has_overhang) or z=0 if none.
ivec4 best_t_slot(const in uint rows[20], const in int heights[10]) {
    ivec4 best = ivec4(0, 0, 0, 0);
    for (int x = 1; x < 9; x++) {
        int stem = heights[x];
        if (stem + 1 > heights[x - 1] || stem + 1 > heights[x + 1]) continue;
        int y = stem;
        int bx = x - 1;
        if (!can_place_board(rows, 2u, 2u, bx, y)) continue;
        if (can_place_board(rows, 2u, 2u, bx, y - 1)) continue;
        // Rotation::Two T cells: (1,0), (0,1), (1,1), (2,1)
        uint rmask0 = 1u << uint(bx + 1);
        uint rmask1 = (7u << uint(bx));
        int lines = 0;
        if (y >= 0 && y < 40 && (get_row(rows, uint(y)) | rmask0) == FULL_ROW) lines++;
        if (y + 1 >= 0 && y + 1 < 40 && (get_row(rows, uint(y + 1)) | rmask1) == FULL_ROW) lines++;
        if (lines == 0) continue;
        int has_oh = (corner_filled(rows, bx, y + 2) || corner_filled(rows, bx + 2, y + 2)) ? 1 : 0;
        int score = lines * 2 + has_oh;
        int best_score = best.z * 2 + best.w;
        if (score > best_score) {
            best = ivec4(bx, y, lines, has_oh);
        }
    }
    return best;
}

float t_slot_cutout_val(const in uint rows[20], const in int heights[10], uint t_avail, const in EvalWeights w) {
    if (t_avail == 0u) return 0.0;
    ivec4 first = best_t_slot(rows, heights);
    if (first.z == 0) return 0.0;
    float r1 = (first.w != 0) ? 1.0 : w.t_slot_open_ratio;
    float val = ((first.z == 2) ? w.t_slot2 : w.t_slot1) * r1;
    if (t_avail < 2u) return val;
    // Place first T and clear lines on a local copy to check second T-slot.
    uint tmp[20];
    for (uint i = 0u; i < 20u; i++) tmp[i] = rows[i];
    set_row(tmp, uint(first.y), get_row(tmp, uint(first.y)) | (1u << uint(first.x + 1)));
    set_row(tmp, uint(first.y + 1), get_row(tmp, uint(first.y + 1)) | (7u << uint(first.x)));
    uint wr = 0u;
    for (uint rd = 0u; rd < 40u; rd++) {
        uint rv = get_row(tmp, rd);
        if (rv != FULL_ROW) {
            if (wr != rd) set_row(tmp, wr, rv);
            wr++;
        }
    }
    for (uint y = wr; y < 40u; y++) set_row(tmp, y, 0u);
    int h2[10];
    for (int x = 0; x < 10; x++) h2[x] = 0;
    uint top = board_height(tmp);
    for (int y = int(top) - 1; y >= 0; y--) {
        uint rv = get_row(tmp, uint(y));
        while (rv != 0u) {
            int x = findLSB(rv);
            rv &= rv - 1u;
            if (h2[x] == 0) h2[x] = y + 1;
        }
    }
    ivec4 second = best_t_slot(tmp, h2);
    float r2 = (second.w != 0) ? 1.0 : w.t_slot_open_ratio;
    if (second.z == 2) val += w.t_slot2 * r2;
    else if (second.z == 1) val += w.t_slot1 * r2;
    return val;
}

float eval_leaf(
    const in uint rows[20],
    uint glo,
    uint ghi,
    uint b2b_streak,
    uint combo,
    uint incoming,
    uint arrived,
    uint sent_total,
    uint received_total,
    float multiplier,
    uint t_avail,
    uint hold_kind,
    const in EvalWeights w
) {
    uint top = board_height(rows);
    int heights[10];
    uint cover[10];
    for (uint x = 0u; x < 10u; x++) { heights[x] = 0; cover[x] = 0u; }
    uint seen = 0u;
    float f_holes = 0.0;
    float f_hole_cover = 0.0;
    float f_hole_rows = 0.0;
    for (int y = int(top) - 1; y >= 0; y--) {
        uint row = get_row(rows, uint(y));
        uint hmask = (~row) & seen & FULL_ROW;
        if (hmask != 0u) {
            f_hole_rows += 1.0;
            while (hmask != 0u) {
                uint x = uint(findLSB(hmask));
                hmask &= hmask - 1u;
                f_holes += 1.0;
                f_hole_cover += float(min(cover[x], 4u));
            }
        }
        uint filled = row;
        while (filled != 0u) {
            uint x = uint(findLSB(filled));
            filled &= filled - 1u;
            if (heights[x] == 0) heights[x] = y + 1;
            cover[x] = min(cover[x] + 1u, 4u);
        }
        seen |= row;
    }

    int max_h = 0;
    for (uint x = 0u; x < 10u; x++) max_h = max(max_h, heights[x]);
    int over_h = max(max_h - DANGER_BASE, 0);
    float f_max_height = float(max_h);
    float f_height_sq = float(over_h * over_h);

    float f_quad_ready = 0.0;
    for (int x = 1; x < 9; x++) {
        if (heights[x] + 4 <= heights[x - 1] && heights[x] + 4 <= heights[x + 1]) {
            bool solid = true;
            for (int r = 0; r < heights[x]; r++) {
                if ((get_row(rows, uint(r)) & (1u << uint(x))) == 0u) { solid = false; break; }
            }
            if (solid) { f_quad_ready = 1.0; break; }
        }
    }

    float f_danger_height = 0.0;
    for (uint x = 0u; x < 10u; x++) {
        int ov = max(heights[x] - int(DANGER_THRESHOLD), 0);
        f_danger_height += float(ov * ov);
    }

    int well = 0;
    for (int x = 1; x < 10; x++) {
        if (heights[x] < heights[well]) well = x;
    }
    int wleft = (well == 0) ? 40 : heights[well - 1];
    int wright = (well == 9) ? 40 : heights[well + 1];
    float f_well_depth = float(clamp(min(wleft, wright) - heights[well], 0, 4));

    float f_bump = 0.0;
    float f_bump_sq = 0.0;
    float f_extra_wells = 0.0;
    for (int x = 0; x < 10; x++) {
        int h = heights[x];
        int l = (x == 0) ? 0 : heights[x - 1];
        int r = (x == 9) ? 0 : heights[x + 1];
        bool touches_well = (x == well || x + 1 == well);
        if (!touches_well) {
            int d = abs(h - r);
            f_bump += float(d);
            f_bump_sq += float(d * d);
        }
        if (x != well && h + 3 <= l && h + 3 <= r) {
            f_extra_wells += 1.0;
        }
    }

    float f_row_trans = 0.0;
    float f_col_trans = 0.0;
    float f_grows = 0.0;
    uint prev_row = FULL_ROW;
    for (uint y = 0u; y < top; y++) {
        uint row = get_row(rows, y);
        f_col_trans += float(bitCount(prev_row ^ row));
        prev_row = row;
        if (get_gbit(glo, ghi, y)) f_grows += 1.0;
        if (row == 0u) continue;
        uint mask = (row << 1u) | 0x801u;
        f_row_trans += float(bitCount(mask ^ (mask >> 1u)));
    }
    if (top > 0u) {
        f_col_trans += float(bitCount(prev_row));
    }

    // Garbage hole cover: topmost (and 2nd distinct) garbage hole columns
    float f_gcover = 0.0;
    int g_hole1 = -1;
    for (int y = int(top) - 1; y >= 0; y--) {
        if (!get_gbit(glo, ghi, uint(y))) continue;
        uint row = get_row(rows, uint(y));
        if (bitCount(row) == 9) {
            uint miss = (~row) & FULL_ROW;
            int x = findLSB(miss);
            float cov = float(clamp(heights[x] - (y + 1), 0, 6));
            if (g_hole1 < 0) {
                g_hole1 = x;
                f_gcover += cov;
            } else if (x != g_hole1) {
                f_gcover += 0.5 * cov;
                break;
            }
        }
    }

    int center_h = max(max(heights[3], heights[4]), max(heights[5], heights[6]));
    int c_over = max(center_h - 10, 0);
    float f_center_sq = float(c_over * c_over);
    bool well_in_center = (well >= 3 && well <= 6 && f_well_depth >= 2.0);

    float wait_pieces = float(max(int(SURVIVAL_PROJECTED) - int(top) - int(incoming), 0));
    float readiness_scale = min(wait_pieces / 4.0, 1.0);
    float height_risk = clamp((f_max_height - w.risk_base) / w.risk_span, w.risk_floor, 1.0);
    float mult = max(multiplier, 1.0);
    float mult_scale = 1.0 + w.height_mult_scale * max(mult - 1.0, 0.0);

    float value = height_risk * mult_scale * (w.max_height * f_max_height + w.height_sq * f_height_sq + w.danger_height * f_danger_height)
        + readiness_scale * (w.quad_ready * f_quad_ready + w.well_depth * f_well_depth)
        + w.holes * f_holes
        + w.hole_cover * f_hole_cover
        + w.hole_rows * f_hole_rows
        + w.bumpiness * f_bump
        + w.bumpiness_sq * f_bump_sq
        + w.row_transitions * f_row_trans
        + w.col_transitions * f_col_trans
        + w.extra_wells * f_extra_wells
        + w.garbage_cover * f_gcover
        + w.center_height * f_center_sq * mult_scale
        + w.downstack_urgency * f_grows * max(f_max_height - 8.0, 0.0) * mult_scale;

    if (well_in_center) {
        value += w.well_center_penalty * readiness_scale;
    }
    if (hold_kind == 2u) {
        value += w.hold_t_bonus * readiness_scale;
    } else if (hold_kind == 0u) {
        value += w.hold_i_bonus * readiness_scale;
    }

    float proj = f_max_height + float(incoming);
    float over_p = max(proj - float(DANGER_BASE), 0.0);
    value += w.danger_projected * over_p * over_p * mult_scale;
    value += w.garbage_rows * f_grows;

    float t_slot_val = t_slot_cutout_val(rows, heights, t_avail, w);
    float charge = float(surge_charge_from_streak(b2b_streak));
    float convert_prob = clamp((18.0 - proj) / 8.0, 0.0, 1.0);
    float growth_gate = clamp((14.0 - f_max_height) / 6.0, 0.0, 1.0)
        * clamp((20.0 - proj) / 6.0, 0.0, 1.0);
    if (charge > 0.0) {
        value += w.surge_charge * pow(charge, w.surge_exponent) * mult * convert_prob;
    }

    float next_charge = float(surge_charge_from_streak(b2b_streak + 1u));
    float delta_surge = max(next_charge - charge, 0.0);
    if (delta_surge > 0.0 && growth_gate > 0.0) {
        float opp = (t_slot_val > 0.0) ? 1.0 : ((f_well_depth >= 4.0) ? 0.7 : 0.35);
        value += 0.5 * w.surge_charge * delta_surge * mult * opp * growth_gate;
    }

    float buildup = float(min(b2b_streak, B2B_CHARGE_AT));
    value += w.b2b_stock * buildup * mult * readiness_scale;
    value += w.margin_stock * max(mult - 1.0, 0.0) * (buildup + charge) * readiness_scale;
    float c_cap = float(min(combo, COMBO_CAP));
    value += w.combo * c_cap;
    value += w.combo_state * log(1.25 * c_cap + 1.0) * mult * max(growth_gate, 0.25);

    float capped = f_max_height + float(min(arrived, GARBAGE_CAP));
    if (capped >= float(SURVIVAL_PROJECTED - 2u)) {
        uint cp = 0u;
        for (uint r = 0u; r < top && cp < 2u; r++) {
            uint rm = get_row(rows, r);
            if (bitCount(rm) == 9) {
                uint miss = (~rm) & FULL_ROW;
                int x = findLSB(miss);
                if (heights[x] == int(r) + 1) cp++;
            }
        }
        value += w.combo_potential * float(cp);
    }

    value -= w.incoming * float(incoming);
    value += w.race * float(int(sent_total) - int(received_total));
    value += readiness_scale * t_slot_val;
    return value;
}

// 64-bit FNV-1a hash split across uvec2(lo, hi)
uvec2 fnv_step(uvec2 h, uint v) {
    // 64-bit FNV prime = 0x100000001b3 -> hi = 0x100, lo = 0x1b3
    h.x ^= v;
    // Multiply (h.x, h.y) by 0x100000001b3
    uint lo_mul_hi = 0u;
    uint lo_mul_lo = 0u;
    umulExtended(h.x, 0x1b3u, lo_mul_hi, lo_mul_lo);
    uint hi_part = lo_mul_hi + h.y * 0x1b3u + (h.x << 8u);
    return uvec2(lo_mul_lo, hi_part);
}
