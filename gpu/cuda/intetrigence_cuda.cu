// Intetrigence CUDA S2 Beam Search Engine (for NVIDIA Colab L4 / A100).
// 100% original implementation matching gpu/shaders/{common.glsl,expand.comp,select.comp}
// and speaking the exact same binary `serve` protocol as gpu/host/intetrigence_gpu.c.
//
// Build: nvcc -O3 -arch=sm_89 gpu/cuda/intetrigence_cuda.cu -o gpu/build/intetrigence_cuda -lm
// Run:   intetrigence_cuda <unused> <unused> <bench [w] [d] [iters] | serve>

#include <cuda_runtime.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define MAX_WIDTH    4096u
#define MAX_CHILDREN 524288u
#define MAX_ROOTS    256u
#define PROTO_MAGIC  0x49475055u
#define FULL_ROW     0x3FFu
#define GARBAGE_CAP  8u
#define OPENER_PIECES 14u
#define COMBO_CAP    4u
#define DANGER_THRESHOLD 12
#define SURVIVAL_PROJECTED 21u
#define DANGER_BASE  15
#define B2B_CHARGE_AT 4u
#define B2B_CHARGE_BASE 3u
#define PC_GARBAGE   5u

#define CUDA_CHECK(call) do { \
    cudaError_t _e = (call); \
    if (_e != cudaSuccess) { \
        fprintf(stderr, "CUDA error %s at %s:%d\n", cudaGetErrorString(_e), __FILE__, __LINE__); \
        exit(1); \
    } \
} while (0)

typedef struct {
    uint32_t rows[20];
    uint32_t gmask_lo;
    uint32_t gmask_hi;
    uint32_t meta0;
    uint32_t meta1;
    uint32_t sent_total;
    uint32_t received_total;
    uint32_t pieces_placed;
    float reward;
    float value;
    uint32_t key_lo;
    uint32_t key_hi;
    uint32_t pad0;
} GpuNode;

typedef struct {
    uint32_t eligible;
    uint32_t hypothetical;
    uint32_t extra;
    float multiplier;
} LockForecast;

typedef struct {
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
    float survival_flag;
    float survival_line;
    float incoming;
    float discount;
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
} EvalWeights;

typedef struct {
    uint32_t seq[24];
    uint32_t seq_len;
    uint32_t depth;
    uint32_t width;
    uint32_t initial_incoming;
    uint32_t projection_mode;
    uint32_t root_count;
    uint32_t pad0;
    uint32_t pad1;
    LockForecast locks[24];
    EvalWeights w;
} SearchParams;

typedef struct {
    uint32_t child_count;
    uint32_t beam_count;
    uint32_t level;
    uint32_t overflow;
    uint32_t dispatch_x;
    uint32_t dispatch_y;
    uint32_t dispatch_z;
    uint32_t pad0;
} RuntimeState;

static_assert(sizeof(GpuNode) == 128, "GpuNode size mismatch");
static_assert(sizeof(EvalWeights) == 288, "EvalWeights size mismatch");
static_assert(sizeof(SearchParams) == 800, "SearchParams size mismatch");
static_assert(sizeof(RuntimeState) == 32, "RuntimeState size mismatch");

__host__ __device__ static inline uint32_t float_to_sortable(float f) {
    uint32_t u;
    memcpy(&u, &f, 4);
    return (u & 0x80000000u) ? ~u : (u | 0x80000000u);
}

__host__ __device__ static inline float sortable_to_float(uint32_t s) {
    uint32_t u = (s & 0x80000000u) ? (s & 0x7FFFFFFFu) : ~s;
    float f;
    memcpy(&f, &u, 4);
    return f;
}

__device__ static inline uint32_t get_row(const uint32_t rows[20], uint32_t y) {
    return (rows[y >> 1u] >> ((y & 1u) * 16u)) & FULL_ROW;
}

__device__ static inline void set_row(uint32_t rows[20], uint32_t y, uint32_t val) {
    uint32_t idx = y >> 1u;
    uint32_t sh = (y & 1u) * 16u;
    rows[idx] = (rows[idx] & ~(0xFFFFu << sh)) | ((val & FULL_ROW) << sh);
}

__device__ static inline bool get_gbit(uint32_t glo, uint32_t ghi, uint32_t y) {
    return (y < 32u) ? (((glo >> y) & 1u) != 0u) : (((ghi >> (y - 32u)) & 1u) != 0u);
}

__device__ static inline void set_gbit(uint32_t &glo, uint32_t &ghi, uint32_t y, bool v) {
    if (y < 32u) {
        if (v) glo |= (1u << y); else glo &= ~(1u << y);
    } else {
        uint32_t bit = 1u << (y - 32u);
        if (v) ghi |= bit; else ghi &= ~bit;
    }
}

__device__ static inline uint32_t board_height(const uint32_t rows[20]) {
    for (int i = 19; i >= 0; i--) {
        uint32_t w = rows[i];
        if (w != 0u) {
            return ((w >> 16u) != 0u) ? (uint32_t)(i * 2 + 2) : (uint32_t)(i * 2 + 1);
        }
    }
    return 0u;
}

__device__ static inline int find_lsb(uint32_t x) {
    return __ffs((int)x) - 1;
}

__device__ static inline int2 piece_cell(uint32_t kind, uint32_t rot, uint32_t i) {
    int2 c = make_int2(0, 0);
    int maxb = (kind == 0u) ? 3 : ((kind == 1u) ? 1 : 2);
    if (kind == 0u) {
        c = make_int2((int)i, 2);
    } else if (kind == 1u) {
        const int ox[4] = {0, 1, 0, 1};
        const int oy[4] = {0, 0, 1, 1};
        c = make_int2(ox[i], oy[i]);
    } else if (kind == 2u) {
        const int tx[4] = {1, 0, 1, 2};
        const int ty[4] = {2, 1, 1, 1};
        c = make_int2(tx[i], ty[i]);
    } else if (kind == 3u) {
        const int sx[4] = {1, 2, 0, 1};
        const int sy[4] = {2, 2, 1, 1};
        c = make_int2(sx[i], sy[i]);
    } else if (kind == 4u) {
        const int zx[4] = {0, 1, 1, 2};
        const int zy[4] = {2, 2, 1, 1};
        c = make_int2(zx[i], zy[i]);
    } else if (kind == 5u) {
        const int jx[4] = {0, 0, 1, 2};
        const int jy[4] = {2, 1, 1, 1};
        c = make_int2(jx[i], jy[i]);
    } else {
        const int lx[4] = {2, 0, 1, 2};
        const int ly[4] = {2, 1, 1, 1};
        c = make_int2(lx[i], ly[i]);
    }
    for (uint32_t s = 0u; s < rot; s++) {
        c = make_int2(c.y, maxb - c.x);
    }
    return c;
}

__device__ static inline int2 spawn_pos(uint32_t kind) {
    if (kind == 0u) return make_int2(3, 19);
    if (kind == 1u) return make_int2(4, 21);
    return make_int2(3, 20);
}

__device__ static inline int kick90_index(uint32_t from, uint32_t to) {
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

__device__ static inline int kick180_index(uint32_t from, uint32_t to) {
    if (from == 0u && to == 2u) return 0;
    if (from == 2u && to == 0u) return 1;
    if (from == 1u && to == 3u) return 2;
    if (from == 3u && to == 1u) return 3;
    return -1;
}

__constant__ int2 d_JLSTZ_K90[40] = {
    {0,0},{-1,0},{-1,1},{0,-2},{-1,-2},
    {0,0},{1,0},{1,-1},{0,2},{1,2},
    {0,0},{1,0},{1,-1},{0,2},{1,2},
    {0,0},{-1,0},{-1,1},{0,-2},{-1,-2},
    {0,0},{1,0},{1,1},{0,-2},{1,-2},
    {0,0},{-1,0},{-1,-1},{0,2},{-1,2},
    {0,0},{-1,0},{-1,-1},{0,2},{-1,2},
    {0,0},{1,0},{1,1},{0,-2},{1,-2}
};

__constant__ int2 d_I_K90_SRSP[40] = {
    {0,0},{1,0},{-2,0},{-2,-1},{1,2},
    {0,0},{-1,0},{2,0},{-1,-2},{2,1},
    {0,0},{-1,0},{2,0},{-1,2},{2,-1},
    {0,0},{-2,0},{1,0},{-2,1},{1,-2},
    {0,0},{2,0},{-1,0},{2,1},{-1,-2},
    {0,0},{1,0},{-2,0},{1,2},{-2,-1},
    {0,0},{1,0},{-2,0},{1,-2},{-2,1},
    {0,0},{-1,0},{2,0},{2,-1},{-1,2}
};

__constant__ int2 d_JLSTZ_K180[24] = {
    {0,0},{0,1},{1,1},{-1,1},{1,0},{-1,0},
    {0,0},{0,-1},{-1,-1},{1,-1},{-1,0},{1,0},
    {0,0},{1,0},{1,2},{1,1},{0,2},{0,1},
    {0,0},{-1,0},{-1,2},{-1,1},{0,2},{0,1}
};

__constant__ int2 d_I_K180[8] = {
    {0,0},{0,1},{0,0},{0,-1},{0,0},{1,0},{0,0},{-1,0}
};

__device__ static inline bool is_fin_or_tst(uint32_t from, uint32_t to, int2 d) {
    if (abs(d.x) == 1 && abs(d.y) == 2) return true;
    return (from == 0u && to == 2u && d.x == 0 && d.y == 1) ||
           (from == 2u && to == 0u && d.x == 0 && d.y == -1);
}

__device__ static inline bool can_place_board(const uint32_t rows[20], uint32_t kind, uint32_t rot, int x, int y) {
    for (uint32_t i = 0u; i < 4u; i++) {
        int2 c = piece_cell(kind, rot, i);
        int bx = x + c.x;
        int by = y + c.y;
        if (bx < 0 || bx >= 10 || by < 0 || by >= 40) return false;
        if ((get_row(rows, (uint32_t)by) & (1u << (uint32_t)bx)) != 0u) return false;
    }
    return true;
}

__device__ static inline bool is_immobile_board(const uint32_t rows[20], uint32_t kind, uint32_t rot, int x, int y) {
    return !can_place_board(rows, kind, rot, x - 1, y) &&
           !can_place_board(rows, kind, rot, x + 1, y) &&
           !can_place_board(rows, kind, rot, x, y + 1) &&
           !can_place_board(rows, kind, rot, x, y - 1);
}

__device__ static inline bool corner_occupied(const uint32_t rows[20], int x, int y) {
    if (x < 0 || x >= 10 || y < 0) return true;
    if (y >= 40) return false;
    return (get_row(rows, (uint32_t)y) & (1u << (uint32_t)x)) != 0u;
}

__device__ static inline uint32_t classify_spin(const uint32_t rows[20], uint32_t kind, uint32_t rot, int x, int y, bool spin_entry, bool fin_or_tst) {
    if (!spin_entry) return 0u;
    if (kind == 2u) {
        int cx = x + 1, cy = y + 1;
        bool tl = corner_occupied(rows, cx - 1, cy + 1);
        bool tr = corner_occupied(rows, cx + 1, cy + 1);
        bool bl = corner_occupied(rows, cx - 1, cy - 1);
        bool br = corner_occupied(rows, cx + 1, cy - 1);
        uint32_t corners = (uint32_t)tl + (uint32_t)tr + (uint32_t)bl + (uint32_t)br;
        uint32_t front = 0u;
        if (rot == 0u) front = (uint32_t)tl + (uint32_t)tr;
        else if (rot == 1u) front = (uint32_t)tr + (uint32_t)br;
        else if (rot == 2u) front = (uint32_t)bl + (uint32_t)br;
        else front = (uint32_t)tl + (uint32_t)bl;
        if (corners >= 3u) return (front == 2u || fin_or_tst) ? 2u : 1u;
    }
    return is_immobile_board(rows, kind, rot, x, y) ? 1u : 0u;
}

__device__ static inline uint32_t surge_charge_from_streak(uint32_t b2b) {
    uint32_t disp = (b2b > 0u) ? (b2b - 1u) : 0u;
    if (disp >= B2B_CHARGE_AT) return disp - B2B_CHARGE_AT + B2B_CHARGE_BASE + 1u;
    return 0u;
}

__device__ static inline uint32_t base_attack_val(uint32_t spin, uint32_t lines) {
    const uint32_t tbl[15] = {
        0u, 0u, 1u, 2u, 4u,
        0u, 0u, 1u, 2u, 10u,
        0u, 2u, 4u, 6u, 10u
    };
    return tbl[spin * 5u + lines];
}

__device__ static inline int4 best_t_slot(const uint32_t rows[20], const int heights[10]) {
    int4 best = make_int4(0, 0, 0, 0);
    for (int x = 1; x < 9; x++) {
        int stem = heights[x];
        if (stem + 1 > heights[x - 1] || stem + 1 > heights[x + 1]) continue;
        int y = stem;
        int bx = x - 1;
        if (!can_place_board(rows, 2u, 2u, bx, y)) continue;
        if (can_place_board(rows, 2u, 2u, bx, y - 1)) continue;
        uint32_t rmask0 = 1u << (uint32_t)(bx + 1);
        uint32_t rmask1 = (7u << (uint32_t)bx);
        int lines = 0;
        if (y >= 0 && y < 40 && (get_row(rows, (uint32_t)y) | rmask0) == FULL_ROW) lines++;
        if (y + 1 >= 0 && y + 1 < 40 && (get_row(rows, (uint32_t)(y + 1)) | rmask1) == FULL_ROW) lines++;
        if (lines == 0) continue;
        int has_oh = (corner_occupied(rows, bx, y + 2) || corner_occupied(rows, bx + 2, y + 2)) ? 1 : 0;
        if (lines * 2 + has_oh > best.z * 2 + best.w) {
            best = make_int4(bx, y, lines, has_oh);
        }
    }
    return best;
}

__device__ static inline float t_slot_cutout_val(const uint32_t rows[20], const int heights[10], uint32_t t_avail, const EvalWeights &w) {
    if (t_avail == 0u) return 0.0f;
    int4 first = best_t_slot(rows, heights);
    if (first.z == 0) return 0.0f;
    float r1 = (first.w != 0) ? 1.0f : w.t_slot_open_ratio;
    float val = ((first.z == 2) ? w.t_slot2 : w.t_slot1) * r1;
    if (t_avail < 2u) return val;
    uint32_t tmp[20];
    for (uint32_t i = 0u; i < 20u; i++) tmp[i] = rows[i];
    set_row(tmp, (uint32_t)first.y, get_row(tmp, (uint32_t)first.y) | (1u << (uint32_t)(first.x + 1)));
    set_row(tmp, (uint32_t)(first.y + 1), get_row(tmp, (uint32_t)(first.y + 1)) | (7u << (uint32_t)first.x));
    uint32_t wr = 0u;
    for (uint32_t rd = 0u; rd < 40u; rd++) {
        uint32_t rv = get_row(tmp, rd);
        if (rv != FULL_ROW) {
            if (wr != rd) set_row(tmp, wr, rv);
            wr++;
        }
    }
    for (uint32_t y = wr; y < 40u; y++) set_row(tmp, y, 0u);
    int h2[10] = {0,0,0,0,0,0,0,0,0,0};
    uint32_t top = board_height(tmp);
    for (int y = (int)top - 1; y >= 0; y--) {
        uint32_t rv = get_row(tmp, (uint32_t)y);
        while (rv != 0u) {
            int x = find_lsb(rv);
            rv &= rv - 1u;
            if (h2[x] == 0) h2[x] = y + 1;
        }
    }
    int4 second = best_t_slot(tmp, h2);
    float r2 = (second.w != 0) ? 1.0f : w.t_slot_open_ratio;
    if (second.z == 2) val += w.t_slot2 * r2;
    else if (second.z == 1) val += w.t_slot1 * r2;
    return val;
}

__device__ static inline float clampf(float v, float lo, float hi) {
    return fminf(fmaxf(v, lo), hi);
}

__device__ static inline float eval_leaf(
    const uint32_t rows[20],
    uint32_t glo,
    uint32_t ghi,
    uint32_t b2b_streak,
    uint32_t combo,
    uint32_t incoming,
    uint32_t arrived,
    uint32_t sent_total,
    uint32_t received_total,
    float multiplier,
    uint32_t t_avail,
    uint32_t hold_kind,
    const EvalWeights &w
) {
    uint32_t top = board_height(rows);
    int heights[10] = {0,0,0,0,0,0,0,0,0,0};
    uint32_t cover[10] = {0,0,0,0,0,0,0,0,0,0};
    uint32_t seen = 0u;
    float f_holes = 0.0f, f_hole_cover = 0.0f, f_hole_rows = 0.0f;
    for (int y = (int)top - 1; y >= 0; y--) {
        uint32_t row = get_row(rows, (uint32_t)y);
        uint32_t hmask = (~row) & seen & FULL_ROW;
        if (hmask != 0u) {
            f_hole_rows += 1.0f;
            while (hmask != 0u) {
                uint32_t x = (uint32_t)find_lsb(hmask);
                hmask &= hmask - 1u;
                f_holes += 1.0f;
                f_hole_cover += (float)(cover[x] < 4u ? cover[x] : 4u);
            }
        }
        uint32_t filled = row;
        while (filled != 0u) {
            uint32_t x = (uint32_t)find_lsb(filled);
            filled &= filled - 1u;
            if (heights[x] == 0) heights[x] = y + 1;
            cover[x] = (cover[x] + 1u < 4u) ? (cover[x] + 1u) : 4u;
        }
        seen |= row;
    }

    int max_h = 0;
    for (int x = 0; x < 10; x++) if (heights[x] > max_h) max_h = heights[x];
    int over_h = max_h > DANGER_BASE ? (max_h - DANGER_BASE) : 0;
    float f_max_height = (float)max_h;
    float f_height_sq = (float)(over_h * over_h);

    float f_quad_ready = 0.0f;
    for (int x = 1; x < 9; x++) {
        if (heights[x] + 4 <= heights[x - 1] && heights[x] + 4 <= heights[x + 1]) {
            bool solid = true;
            for (int r = 0; r < heights[x]; r++) {
                if ((get_row(rows, (uint32_t)r) & (1u << (uint32_t)x)) == 0u) { solid = false; break; }
            }
            if (solid) { f_quad_ready = 1.0f; break; }
        }
    }

    float f_danger_height = 0.0f;
    for (int x = 0; x < 10; x++) {
        int ov = heights[x] > DANGER_THRESHOLD ? (heights[x] - DANGER_THRESHOLD) : 0;
        f_danger_height += (float)(ov * ov);
    }

    int well = 0;
    for (int x = 1; x < 10; x++) if (heights[x] < heights[well]) well = x;
    int wleft = (well == 0) ? 40 : heights[well - 1];
    int wright = (well == 9) ? 40 : heights[well + 1];
    int wd = (wleft < wright ? wleft : wright) - heights[well];
    if (wd < 0) wd = 0; else if (wd > 4) wd = 4;
    float f_well_depth = (float)wd;

    float f_bump = 0.0f, f_bump_sq = 0.0f, f_extra_wells = 0.0f;
    for (int x = 0; x < 10; x++) {
        int h = heights[x];
        int l = (x == 0) ? 0 : heights[x - 1];
        int r = (x == 9) ? 0 : heights[x + 1];
        bool touches_well = (x == well || x + 1 == well);
        if (!touches_well) {
            int d = abs(h - r);
            f_bump += (float)d;
            f_bump_sq += (float)(d * d);
        }
        if (x != well && h + 3 <= l && h + 3 <= r) f_extra_wells += 1.0f;
    }

    float f_row_trans = 0.0f, f_col_trans = 0.0f, f_grows = 0.0f;
    uint32_t prev_row = FULL_ROW;
    for (uint32_t y = 0u; y < top; y++) {
        uint32_t row = get_row(rows, y);
        f_col_trans += (float)__popc(prev_row ^ row);
        prev_row = row;
        if (get_gbit(glo, ghi, y)) f_grows += 1.0f;
        if (row == 0u) continue;
        uint32_t mask = (row << 1u) | 0x801u;
        f_row_trans += (float)__popc(mask ^ (mask >> 1u));
    }
    if (top > 0u) f_col_trans += (float)__popc(prev_row);

    float f_gcover = 0.0f;
    int g_hole1 = -1;
    for (int y = (int)top - 1; y >= 0; y--) {
        if (!get_gbit(glo, ghi, (uint32_t)y)) continue;
        uint32_t row = get_row(rows, (uint32_t)y);
        if (__popc(row) == 9) {
            uint32_t miss = (~row) & FULL_ROW;
            int x = find_lsb(miss);
            int c = heights[x] - (y + 1);
            if (c < 0) c = 0; else if (c > 6) c = 6;
            if (g_hole1 < 0) {
                g_hole1 = x;
                f_gcover += (float)c;
            } else if (x != g_hole1) {
                f_gcover += 0.5f * (float)c;
                break;
            }
        }
    }

    int center_h = heights[3];
    if (heights[4] > center_h) center_h = heights[4];
    if (heights[5] > center_h) center_h = heights[5];
    if (heights[6] > center_h) center_h = heights[6];
    int c_over = center_h > 10 ? (center_h - 10) : 0;
    float f_center_sq = (float)(c_over * c_over);
    bool well_in_center = (well >= 3 && well <= 6 && f_well_depth >= 2.0f);

    int wp = (int)SURVIVAL_PROJECTED - (int)top - (int)incoming;
    float wait_pieces = (float)(wp > 0 ? wp : 0);
    float readiness_scale = fminf(wait_pieces * 0.25f, 1.0f);
    float height_risk = clampf((f_max_height - w.risk_base) / w.risk_span, w.risk_floor, 1.0f);
    float mult = fmaxf(multiplier, 1.0f);
    float mult_scale = 1.0f + w.height_mult_scale * fmaxf(mult - 1.0f, 0.0f);

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
        + w.downstack_urgency * f_grows * fmaxf(f_max_height - 8.0f, 0.0f) * mult_scale;

    if (well_in_center) value += w.well_center_penalty * readiness_scale;
    if (hold_kind == 2u) value += w.hold_t_bonus * readiness_scale;
    else if (hold_kind == 0u) value += w.hold_i_bonus * readiness_scale;

    float proj = f_max_height + (float)incoming;
    float over_p = fmaxf(proj - (float)DANGER_BASE, 0.0f);
    value += w.danger_projected * over_p * over_p * mult_scale;
    value += w.garbage_rows * f_grows;

    float t_slot_val = t_slot_cutout_val(rows, heights, t_avail, w);
    float charge = (float)surge_charge_from_streak(b2b_streak);
    float convert_prob = clampf((18.0f - proj) / 8.0f, 0.0f, 1.0f);
    float growth_gate = clampf((14.0f - f_max_height) / 6.0f, 0.0f, 1.0f)
        * clampf((20.0f - proj) / 6.0f, 0.0f, 1.0f);
    if (charge > 0.0f) {
        value += w.surge_charge * powf(charge, w.surge_exponent) * mult * convert_prob;
    }

    float next_charge = (float)surge_charge_from_streak(b2b_streak + 1u);
    float delta_surge = fmaxf(next_charge - charge, 0.0f);
    if (delta_surge > 0.0f && growth_gate > 0.0f) {
        float opp = (t_slot_val > 0.0f) ? 1.0f : ((f_well_depth >= 4.0f) ? 0.7f : 0.35f);
        value += 0.5f * w.surge_charge * delta_surge * mult * opp * growth_gate;
    }

    float buildup = (float)(b2b_streak < B2B_CHARGE_AT ? b2b_streak : B2B_CHARGE_AT);
    value += w.b2b_stock * buildup * mult * readiness_scale;
    value += w.margin_stock * fmaxf(mult - 1.0f, 0.0f) * (buildup + charge) * readiness_scale;
    float c_cap = (float)(combo < COMBO_CAP ? combo : COMBO_CAP);
    value += w.combo * c_cap;
    value += w.combo_state * logf(1.25f * c_cap + 1.0f) * mult * fmaxf(growth_gate, 0.25f);

    float capped = f_max_height + (float)(arrived < GARBAGE_CAP ? arrived : GARBAGE_CAP);
    if (capped >= (float)(SURVIVAL_PROJECTED - 2u)) {
        uint32_t cp = 0u;
        for (uint32_t r = 0u; r < top && cp < 2u; r++) {
            uint32_t rm = get_row(rows, r);
            if (__popc(rm) == 9) {
                uint32_t miss = (~rm) & FULL_ROW;
                int x = find_lsb(miss);
                if (heights[x] == (int)r + 1) cp++;
            }
        }
        value += w.combo_potential * (float)cp;
    }

    value -= w.incoming * (float)incoming;
    value += w.race * (float)((int)sent_total - (int)received_total);
    value += readiness_scale * t_slot_val;
    return value;
}

__device__ static inline uint64_t fnv_step64(uint64_t h, uint32_t v) {
    h ^= (uint64_t)v;
    return h * 0x100000001b3ULL;
}

__device__ static inline uint32_t shift_x(uint32_t mask, int delta) {
    if (delta >= 0) return (mask << (uint32_t)delta) & 0x7FFFu;
    return mask >> (uint32_t)(-delta);
}

__device__ static inline void insert_garbage_rows(uint32_t rows[20], uint32_t &glo, uint32_t &ghi, uint32_t count, uint32_t hole, uint32_t proj_mode) {
    for (int y = 39; y >= 0; y--) {
        if (y >= (int)count) {
            uint32_t src = (uint32_t)y - count;
            set_row(rows, (uint32_t)y, get_row(rows, src));
            set_gbit(glo, ghi, (uint32_t)y, get_gbit(glo, ghi, src));
        } else {
            uint32_t h = hole;
            if (proj_mode == 2u && count >= 6u && (uint32_t)y >= count / 2u) {
                h = (hole + 5u) % 10u;
            }
            set_row(rows, (uint32_t)y, FULL_ROW & ~(1u << h));
            set_gbit(glo, ghi, (uint32_t)y, true);
        }
    }
}

__global__ void expand_kernel(
    const SearchParams *params_ptr,
    const GpuNode *src_beam,
    GpuNode *children,
    RuntimeState *state,
    uint32_t *root_best
) {
    __shared__ GpuNode s_parent;
    __shared__ uint32_t s_valid[4][45];
    __shared__ uint32_t s_any[4][45];
    __shared__ uint32_t s_rot[4][45];
    __shared__ uint32_t s_fin[4][45];
    __shared__ uint32_t s_rest[128];
    __shared__ uint32_t s_rest_count;
    __shared__ uint32_t s_parent_tslot;
    __shared__ uint32_t s_piece;
    __shared__ uint32_t s_next;
    __shared__ uint32_t s_hold;
    __shared__ uint32_t s_t_avail;
    __shared__ uint32_t s_valid_wg;
    __shared__ uint32_t s_top_yi;

    uint32_t tid = threadIdx.x;
    uint32_t wg = blockIdx.x;
    uint32_t node_idx = wg >> 1u;
    uint32_t use_hold = wg & 1u;
    const SearchParams &params = *params_ptr;

    if (tid == 0u) {
        s_valid_wg = 0u;
        s_rest_count = 0u;
        if (node_idx < state->beam_count) {
            s_parent = src_beam[node_idx];
            uint32_t p_next = s_parent.meta0 & 0xFFu;
            uint32_t p_hold = (s_parent.meta0 >> 8u) & 0xFFu;
            bool can_hold = ((s_parent.meta1 >> 24u) & 2u) != 0u;
            bool is_term = ((s_parent.meta1 >> 24u) & 1u) != 0u;
            if (!is_term && p_next < params.seq_len) {
                if (use_hold == 0u) {
                    s_piece = params.seq[p_next];
                    s_next = p_next + 1u;
                    s_hold = p_hold;
                    s_valid_wg = 1u;
                } else if (can_hold) {
                    if (p_hold < 7u) {
                        s_piece = p_hold;
                        s_next = p_next + 1u;
                        s_hold = params.seq[p_next];
                        s_valid_wg = 1u;
                    } else if (p_next + 1u < params.seq_len) {
                        s_piece = params.seq[p_next + 1u];
                        s_next = p_next + 2u;
                        s_hold = params.seq[p_next];
                        s_valid_wg = 1u;
                    }
                }
            } else if (!is_term && p_next == params.seq_len && use_hold == 1u && can_hold && p_hold < 7u) {
                s_piece = p_hold;
                s_next = p_next;
                s_hold = 255u;
                s_valid_wg = 1u;
            }
            if (s_valid_wg != 0u) {
                uint32_t ta = (s_hold == 2u) ? 1u : 0u;
                for (uint32_t i = s_next; i < params.seq_len; i++) {
                    if (params.seq[i] == 2u) ta++;
                }
                s_t_avail = ta;
                int ph[10] = {0,0,0,0,0,0,0,0,0,0};
                uint32_t bh = board_height(s_parent.rows);
                for (int y = (int)bh - 1; y >= 0; y--) {
                    uint32_t rv = get_row(s_parent.rows, (uint32_t)y);
                    while (rv != 0u) {
                        int x = find_lsb(rv);
                        rv &= rv - 1u;
                        if (ph[x] == 0) ph[x] = y + 1;
                    }
                }
                s_parent_tslot = (uint32_t)best_t_slot(s_parent.rows, ph).z;
                int2 sp = spawn_pos(s_piece);
                int top_y = (bh + 6u >= 40u) ? 40 : min(max((int)bh + 6, sp.y + 4), 40);
                int tyi = top_y + 4;
                if (tyi < 0) tyi = 0; else if (tyi > 44) tyi = 44;
                s_top_yi = (uint32_t)tyi;
            }
        }
    }
    __syncthreads();
    if (s_valid_wg == 0u) return;

    uint32_t kind = s_piece;
    uint32_t top_yi = s_top_yi;

    if (tid <= top_yi) {
        int y = (int)tid - 4;
        for (uint32_t r = 0u; r < 4u; r++) {
            uint32_t mask = 0x7FFFu;
            for (uint32_t ci = 0u; ci < 4u; ci++) {
                int2 c = piece_cell(kind, r, ci);
                int by = y + c.y;
                if (by < 0 || by >= 40) { mask = 0u; break; }
                uint32_t free_c = (~get_row(s_parent.rows, (uint32_t)by)) & FULL_ROW;
                mask &= free_c << (uint32_t)(4 - c.x);
            }
            s_valid[r][tid] = mask;
            s_any[r][tid] = 0u;
            s_rot[r][tid] = 0u;
            s_fin[r][tid] = 0u;
        }
    }
    __syncthreads();

    if (tid == 0u) {
        int2 sp = spawn_pos(kind);
        uint32_t sp_yi = (uint32_t)(sp.y + 4);
        uint32_t sp_bit = 1u << (uint32_t)(sp.x + 4);
        if (sp_yi <= top_yi && (s_valid[0][sp_yi] & sp_bit) != 0u) {
            s_any[0][sp_yi] = sp_bit;
            for (uint32_t iter = 0u; iter < 64u; iter++) {
                bool changed = false;
                for (int yi = (int)top_yi; yi >= 0; yi--) {
                    for (uint32_t r = 0u; r < 4u; r++) {
                        uint32_t v = s_valid[r][yi];
                        uint32_t a = s_any[r][yi];
                        if (v == 0u || a == 0u) continue;
                        while (true) {
                            uint32_t nxt = a | (((a >> 1u) | (a << 1u)) & v);
                            if (nxt == a) break;
                            a = nxt;
                        }
                        s_any[r][yi] = a;
                        if (yi > 0) s_any[r][yi - 1] |= a & s_valid[r][yi - 1];
                        uint32_t targets[3] = { (r + 1u) & 3u, (r + 3u) & 3u, (r + 2u) & 3u };
                        for (uint32_t ti = 0u; ti < 3u; ti++) {
                            uint32_t to = targets[ti];
                            uint32_t rem = a;
                            int k90 = kick90_index(r, to);
                            int k180 = kick180_index(r, to);
                            uint32_t klen = (k90 >= 0) ? ((kind == 1u) ? 1u : 5u) : ((kind == 0u) ? 2u : 6u);
                            for (uint32_t ki = 0u; ki < klen; ki++) {
                                int2 kd = make_int2(0, 0);
                                if (k90 >= 0) {
                                    if (kind == 0u) kd = d_I_K90_SRSP[(uint32_t)k90 * 5u + ki];
                                    else if (kind != 1u) kd = d_JLSTZ_K90[(uint32_t)k90 * 5u + ki];
                                } else {
                                    if (kind == 0u) kd = d_I_K180[(uint32_t)k180 * 2u + ki];
                                    else kd = d_JLSTZ_K180[(uint32_t)k180 * 6u + ki];
                                }
                                int nyi = yi + kd.y;
                                if (nyi < 0 || nyi > (int)top_yi) continue;
                                uint32_t src_ok = shift_x(s_valid[to][nyi], -kd.x);
                                uint32_t hit = rem & src_ok;
                                if (hit != 0u) {
                                    rem &= ~hit;
                                    uint32_t dest = shift_x(hit, kd.x);
                                    if ((dest & ~s_any[to][nyi]) != 0u) {
                                        s_any[to][nyi] |= dest;
                                        if (nyi > yi || (nyi == yi && to < r)) changed = true;
                                    }
                                    if (is_fin_or_tst(r, to, kd)) s_fin[to][nyi] |= dest;
                                    else s_rot[to][nyi] |= dest;
                                    if (rem == 0u) break;
                                }
                            }
                        }
                    }
                }
                if (!changed) break;
            }
        }
    }
    __syncthreads();

    if (tid <= top_yi) {
        for (uint32_t r = 0u; r < 4u; r++) {
            uint32_t below = (tid > 0u) ? s_valid[r][tid - 1u] : 0u;
            uint32_t rest = s_any[r][tid] & ~below;
            while (rest != 0u) {
                uint32_t xi = (uint32_t)find_lsb(rest);
                rest &= rest - 1u;
                uint32_t bit = 1u << xi;
                uint32_t is_fin = ((s_fin[r][tid] & bit) != 0u) ? 1u : 0u;
                uint32_t spin_e = (is_fin != 0u || ((s_rot[r][tid] & bit) != 0u)) ? 1u : 0u;
                uint32_t slot = atomicAdd(&s_rest_count, 1u);
                if (slot < 128u) {
                    s_rest[slot] = r | (xi << 2u) | (tid << 6u) | (spin_e << 12u) | (is_fin << 13u);
                }
            }
        }
    }
    __syncthreads();

    uint32_t rcount = min(s_rest_count, 128u);
    for (uint32_t rslot = tid; rslot < rcount; rslot += 64u) {
        uint32_t packed = s_rest[rslot];
        uint32_t rot = packed & 3u;
        int px = (int)((packed >> 2u) & 0xFu) - 4;
        int py = (int)((packed >> 6u) & 0x3Fu) - 4;
        bool spin_entry = ((packed >> 12u) & 1u) != 0u;
        bool fin_or_tst = ((packed >> 13u) & 1u) != 0u;

        uint32_t rows[20];
        for (uint32_t i = 0u; i < 20u; i++) rows[i] = s_parent.rows[i];
        uint32_t glo = s_parent.gmask_lo;
        uint32_t ghi = s_parent.gmask_hi;

        uint32_t spin = classify_spin(rows, kind, rot, px, py, spin_entry, fin_or_tst);
        uint32_t pre_h = board_height(rows);

        for (uint32_t ci = 0u; ci < 4u; ci++) {
            int2 c = piece_cell(kind, rot, ci);
            uint32_t bx = (uint32_t)(px + c.x);
            uint32_t by = (uint32_t)(py + c.y);
            set_row(rows, by, get_row(rows, by) | (1u << bx));
        }

        uint32_t cleared = 0u, gcleared = 0u, wr = 0u;
        for (uint32_t rd = 0u; rd < 40u; rd++) {
            uint32_t rv = get_row(rows, rd);
            if (rv == FULL_ROW) {
                cleared++;
                if (get_gbit(glo, ghi, rd)) gcleared++;
            } else {
                if (wr != rd) {
                    set_row(rows, wr, rv);
                    set_gbit(glo, ghi, wr, get_gbit(glo, ghi, rd));
                }
                wr++;
            }
        }
        for (uint32_t y = wr; y < 40u; y++) {
            set_row(rows, y, 0u);
            set_gbit(glo, ghi, y, false);
        }
        uint32_t post_h = board_height(rows);
        bool pc = (cleared > 0u && post_h == 0u);

        uint32_t b2b_before = (s_parent.meta0 >> 16u) & 0xFFu;
        uint32_t combo_before = (s_parent.meta0 >> 24u) & 0xFFu;
        uint32_t inc = s_parent.meta1 & 0xFFu;
        uint32_t arr = (s_parent.meta1 >> 8u) & 0xFFu;
        uint32_t root_idx = (s_parent.meta1 >> 16u) & 0xFFu;
        uint32_t sent_tot = s_parent.sent_total;
        uint32_t recv_tot = s_parent.received_total;
        uint32_t placed_before = s_parent.pieces_placed;

        uint32_t level = state->level;
        float mult = fmaxf(params.locks[level].multiplier, 1.0f);

        bool difficult = (spin != 0u || cleared >= 4u);
        uint32_t combo_after = 0u;
        uint32_t b2b_after = b2b_before;
        bool broke_b2b = false;
        if (cleared > 0u) {
            combo_after = combo_before + 1u;
            if (pc || difficult) b2b_after = b2b_before + 1u;
            else { b2b_after = 0u; broke_b2b = (b2b_before >= 1u); }
        }

        float base = (float)base_attack_val(spin, cleared);
        if (cleared > 0u && b2b_after >= 2u) base += 1.0f;
        if (cleared > 0u) {
            if (combo_before >= 1u) base *= (1.0f + 0.25f * (float)combo_before);
            if (combo_before >= 2u) base = fmaxf(base, logf(1.25f * (float)combo_before + 1.0f));
        }
        uint32_t gsb = (gcleared > 0u && difficult) ? 1u : 0u;
        uint32_t main_att = (base > 0.0f || gsb > 0u) ? (uint32_t)floorf(base * mult + (float)gsb) : 0u;

        uint32_t events[5];
        uint32_t nev = 0u;
        uint32_t surge_amt = 0u;
        if (broke_b2b && b2b_before > B2B_CHARGE_AT) {
            surge_amt = (uint32_t)floorf((float)(b2b_before - 1u) * mult);
            if (surge_amt > 0u) {
                uint32_t r = (uint32_t)floorf((float)surge_amt / 3.0f + 0.5f);
                uint32_t c2 = (surge_amt > 2u * r) ? (surge_amt - 2u * r) : 0u;
                if (r > 0u) { events[nev++] = r; events[nev++] = r; }
                if (c2 > 0u) { events[nev++] = c2; }
            }
        }
        if (main_att > 0u) events[nev++] = main_att;
        if (pc) {
            uint32_t pca = (uint32_t)floorf((float)PC_GARBAGE * mult);
            if (pca > 0u) events[nev++] = pca;
        }
        uint32_t att_total = 0u;
        for (uint32_t i = 0u; i < nev; i++) att_total += events[i];

        const EvalWeights &w = params.w;
        uint32_t proj_pre = pre_h + min(arr, GARBAGE_CAP);
        bool survival = (w.survival_flag > 0.5f) && (proj_pre >= SURVIVAL_PROJECTED);
        float rval = w.attack * (float)att_total + w.clear_w[spin * 5u + cleared];
        if (survival && cleared > 0u) rval += w.survival_line * (float)cleared;
        if (cleared > 0u && arr >= 1u) {
            float risk = clampf(((float)proj_pre - 18.0f) / 3.0f, 0.0f, 1.0f);
            rval += w.defense * (float)min(arr, GARBAGE_CAP) * risk;
        }
        if (placed_before < OPENER_PIECES && cleared > 0u) rval += w.opener_clear;
        if (spin == 2u && cleared == 2u) rval += w.tsd_bonus;
        if (spin == 1u) rval += w.mini_penalty;
        if (pc) rval += w.perfect_clear;
        if (broke_b2b) {
            rval += w.b2b_break * mult * ((surge_amt >= 8u) ? w.b2b_keep_mult : 1.0f);
        }
        if (kind == 2u && s_parent_tslot > 0u && spin == 0u) rval += w.wasted_t;
        if (surge_amt > 0u && surge_amt < 8u && proj_pre < 18u) rval += w.surge_hold_penalty;
        if (surge_amt >= 8u) rval += w.surge_cashout_bonus * (float)surge_amt;
        rval += w.combo * (float)combo_after;
        rval += w.garbage_clear * (float)gcleared;
        if (placed_before < OPENER_PIECES) rval += w.opener_attack * (float)att_total;

        uint32_t inc_before = inc;
        uint32_t total_cancelled = 0u;
        for (uint32_t i = 0u; i < nev; i++) {
            uint32_t ev = events[i];
            uint32_t budget = (placed_before < OPENER_PIECES && inc >= sent_tot) ? ev : 0u;
            uint32_t cancelled = min(inc, ev + budget);
            total_cancelled += cancelled;
            inc -= cancelled;
            arr = (arr > cancelled) ? (arr - cancelled) : 0u;
            if (ev > cancelled) sent_tot += (ev - cancelled);
        }
        if (total_cancelled > 0u) {
            float cancel_scale = clampf(((float)(pre_h + inc_before) - 8.0f) / 10.0f, 0.25f, 1.5f);
            rval += w.cancel_bonus * (float)total_cancelled * cancel_scale;
        }

        bool topped_out = (post_h > 20u);
        uint32_t inserted = (cleared == 0u && !topped_out) ? min(min(arr, inc), GARBAGE_CAP) : 0u;
        inc -= inserted;
        arr -= inserted;
        recv_tot += inserted;

        uint32_t next_lvl = level + 1u;
        float next_mult = mult;
        if (next_lvl < 24u) {
            LockForecast lf = params.locks[next_lvl];
            inc += lf.extra;
            uint32_t tot_in = params.initial_incoming + lf.hypothetical;
            uint32_t removed = (tot_in > inc) ? (tot_in - inc) : 0u;
            arr = min((lf.eligible > removed) ? (lf.eligible - removed) : 0u, inc);
            next_mult = lf.multiplier;
        }

        float scale = powf(w.discount, (float)level);
        float acc = s_parent.reward + scale * rval;
        if (post_h > 12u) acc -= scale * w.stack_exposure * (float)(post_h - 12u);

        float leaf = -1e30f;
        if (inserted > 0u) {
            if (post_h + inserted > 20u) {
                topped_out = true;
            } else {
                float vals[10];
                for (uint32_t h = 0u; h < 10u; h++) {
                    uint32_t r2[20];
                    for (uint32_t i = 0u; i < 20u; i++) r2[i] = rows[i];
                    uint32_t glo2 = glo, ghi2 = ghi;
                    insert_garbage_rows(r2, glo2, ghi2, inserted, h, params.projection_mode);
                    vals[h] = eval_leaf(r2, glo2, ghi2, b2b_after, combo_after, inc, arr, sent_tot, recv_tot, next_mult, s_t_avail, s_hold, w);
                }
                if (params.projection_mode == 0u) {
                    float sumv = 0.0f;
                    for (uint32_t h = 0u; h < 10u; h++) sumv += vals[h];
                    leaf = sumv * 0.1f;
                } else {
                    uint32_t cols[10] = {0,1,2,3,4,5,6,7,8,9};
                    for (uint32_t i = 1u; i < 10u; i++) {
                        uint32_t k = cols[i];
                        float kv = vals[k];
                        int j = (int)i - 1;
                        while (j >= 0 && vals[cols[j]] > kv) {
                            cols[j + 1] = cols[j];
                            j--;
                        }
                        cols[j + 1] = k;
                    }
                    uint32_t q_idx = (post_h + inserted >= 18u) ? 1u : ((post_h + inserted >= 15u) ? 2u : 4u);
                    uint32_t med = cols[q_idx];
                    insert_garbage_rows(rows, glo, ghi, inserted, med, params.projection_mode);
                    leaf = vals[med];
                }
            }
        } else if (!topped_out) {
            leaf = eval_leaf(rows, glo, ghi, b2b_after, combo_after, inc, arr, sent_tot, recv_tot, next_mult, s_t_avail, s_hold, w);
        }

        float total_val = topped_out ? -1e30f : (acc + scale * w.discount * leaf);
        bool is_terminal = topped_out || (inserted > 0u && params.projection_mode == 0u) || (s_next >= params.seq_len && s_hold >= 7u);

        if (is_terminal) {
            if (!topped_out) atomicMax(&root_best[root_idx], float_to_sortable(total_val));
            continue;
        }

        uint64_t h = 0xcbf29ce484222325ULL;
        for (uint32_t i = 0u; i < 20u; i++) h = fnv_step64(h, rows[i]);
        h = fnv_step64(h, glo);
        h = fnv_step64(h, ghi);
        h = fnv_step64(h, s_next | (s_hold << 8u) | (b2b_after << 16u) | (combo_after << 24u));
        h = fnv_step64(h, inc | (arr << 8u));

        uint32_t out_idx = atomicAdd(&state->child_count, 1u);
        if (out_idx >= MAX_CHILDREN) {
            atomicAdd(&state->overflow, 1u);
            continue;
        }

        GpuNode child;
        for (uint32_t i = 0u; i < 20u; i++) child.rows[i] = rows[i];
        child.gmask_lo = glo;
        child.gmask_hi = ghi;
        child.meta0 = (s_next & 0xFFu) | ((s_hold & 0xFFu) << 8u) | (min(b2b_after, 255u) << 16u) | (min(combo_after, 255u) << 24u);
        child.meta1 = min(inc, 255u) | (min(arr, 255u) << 8u) | ((root_idx & 0xFFu) << 16u) | (2u << 24u);
        child.sent_total = sent_tot;
        child.received_total = recv_tot;
        child.pieces_placed = placed_before + 1u;
        child.reward = acc;
        child.value = total_val;
        child.key_lo = (uint32_t)(h & 0xFFFFFFFFULL);
        child.key_hi = (uint32_t)(h >> 32u);
        child.pad0 = 0u;
        children[out_idx] = child;
    }
}

#define SEL_CAP 8192u
#define HT_SIZE 16384u
#define HT_MASK (HT_SIZE - 1u)

__global__ void select_kernel(
    const SearchParams *params_ptr,
    const GpuNode *children,
    GpuNode *dst_beam,
    RuntimeState *state,
    uint32_t *root_best,
    uint64_t *g_ht
) {
    extern __shared__ uint32_t s_dyn[];
    uint32_t *s_keys = s_dyn;
    uint32_t *s_idxs = s_dyn + SEL_CAP;
    __shared__ uint32_t s_hist[256];
    __shared__ uint32_t s_b0, s_b1, s_thresh, s_gt_count, s_gather_count, s_kept;

    uint32_t tid = threadIdx.x;
    const SearchParams &params = *params_ptr;
    uint32_t n = min(state->child_count, MAX_CHILDREN);
    uint32_t width = min(params.width, MAX_WIDTH);
    bool final_ply = (state->level + 1u >= params.depth);

    if (n == 0u) {
        if (tid == 0u) {
            state->beam_count = 0u;
            state->child_count = 0u;
            state->level += 1u;
            state->dispatch_x = 0u;
        }
        return;
    }

    if (tid == 0u) { s_gather_count = 0u; s_thresh = 0u; }
    __syncthreads();

    if (n <= SEL_CAP) {
        for (uint32_t i = tid; i < n; i += 256u) {
            s_keys[i] = float_to_sortable(children[i].value);
            s_idxs[i] = i;
        }
        if (tid == 0u) s_gather_count = n;
        __syncthreads();
    } else {
        s_hist[tid] = 0u;
        __syncthreads();
        for (uint32_t i = tid; i < n; i += 256u) {
            uint32_t k = float_to_sortable(children[i].value);
            atomicAdd(&s_hist[k >> 24u], 1u);
        }
        __syncthreads();
        if (tid == 0u) {
            uint32_t acc = 0u, b = 0u;
            for (int i = 255; i >= 0; i--) {
                acc += s_hist[i];
                if (acc >= SEL_CAP) { b = (uint32_t)i; break; }
            }
            s_b0 = b;
            s_gt_count = acc - s_hist[b];
        }
        __syncthreads();

        uint32_t b0 = s_b0;
        s_hist[tid] = 0u;
        __syncthreads();
        for (uint32_t i = tid; i < n; i += 256u) {
            uint32_t k = float_to_sortable(children[i].value);
            if ((k >> 24u) == b0) atomicAdd(&s_hist[(k >> 16u) & 0xFFu], 1u);
        }
        __syncthreads();
        if (tid == 0u) {
            uint32_t acc = s_gt_count, b = 0u;
            for (int i = 255; i >= 0; i--) {
                acc += s_hist[i];
                if (acc >= SEL_CAP) { b = (uint32_t)i; break; }
            }
            s_b1 = b;
            s_thresh = (b0 << 24u) | (b << 16u);
        }
        __syncthreads();

        uint32_t thresh_hi = s_thresh;
        for (uint32_t i = tid; i < n; i += 256u) {
            uint32_t k = float_to_sortable(children[i].value);
            if ((k & 0xFFFF0000u) > thresh_hi) {
                uint32_t pos = atomicAdd(&s_gather_count, 1u);
                if (pos < SEL_CAP) { s_keys[pos] = k; s_idxs[pos] = i; }
            }
        }
        __syncthreads();
        for (uint32_t i = tid; i < n; i += 256u) {
            uint32_t k = float_to_sortable(children[i].value);
            if ((k & 0xFFFF0000u) == thresh_hi) {
                uint32_t pos = atomicAdd(&s_gather_count, 1u);
                if (pos < SEL_CAP) { s_keys[pos] = k; s_idxs[pos] = i; }
            }
        }
        __syncthreads();
    }

    uint32_t m = min(s_gather_count, SEL_CAP);
    for (uint32_t i = m + tid; i < SEL_CAP; i += 256u) {
        s_keys[i] = 0u;
        s_idxs[i] = 0xFFFFFFFFu;
    }
    __syncthreads();

    for (uint32_t size = 2u; size <= SEL_CAP; size <<= 1u) {
        for (uint32_t stride = size >> 1u; stride > 0u; stride >>= 1u) {
            for (uint32_t pair = tid; pair < (SEL_CAP >> 1u); pair += 256u) {
                uint32_t block = pair / stride;
                uint32_t off = pair % stride;
                uint32_t i = block * (stride << 1u) + off;
                uint32_t j = i + stride;
                bool desc = ((i & size) == 0u);
                uint32_t ki = s_keys[i], kj = s_keys[j];
                bool swap = desc ? (ki < kj) : (ki > kj);
                if (swap) {
                    s_keys[i] = kj; s_keys[j] = ki;
                    uint32_t ii = s_idxs[i]; s_idxs[i] = s_idxs[j]; s_idxs[j] = ii;
                }
            }
            __syncthreads();
        }
    }

    for (uint32_t i = tid; i < HT_SIZE; i += 256u) {
        g_ht[i] = 0ULL;
    }
    __syncthreads();

    if (tid == 0u) {
        uint32_t kept = 0u;
        for (uint32_t i = 0u; i < m && kept < width; i++) {
            uint32_t cidx = s_idxs[i];
            if (cidx == 0xFFFFFFFFu) break;
            uint32_t klo = children[cidx].key_lo;
            uint32_t khi = children[cidx].key_hi;
            uint64_t key = ((uint64_t)khi << 32u) | (uint64_t)klo;
            if (key == 0ULL) key = 1ULL;
            uint32_t slot = klo & HT_MASK;
            bool dup = false;
            for (uint32_t probe = 0u; probe < 16u; probe++) {
                uint32_t p = (slot + probe) & HT_MASK;
                uint64_t cur = g_ht[p];
                if (cur == 0ULL) { g_ht[p] = key; break; }
                if (cur == key) { dup = true; break; }
            }
            if (dup) continue;
            s_idxs[kept] = cidx;
            kept++;
        }
        s_kept = kept;
        state->beam_count = kept;
        state->child_count = 0u;
        state->level += 1u;
        state->dispatch_x = final_ply ? 0u : (kept * 2u);
    }
    __syncthreads();

    uint32_t kept = s_kept;
    for (uint32_t i = tid; i < kept; i += 256u) {
        uint32_t cidx = s_idxs[i];
        GpuNode nd = children[cidx];
        dst_beam[i] = nd;
        if (final_ply) {
            uint32_t ridx = (nd.meta1 >> 16u) & 0xFFu;
            atomicMax(&root_best[ridx], float_to_sortable(nd.value));
        }
    }
}

typedef struct {
    SearchParams *d_params;
    GpuNode *d_beam_a;
    GpuNode *d_beam_b;
    GpuNode *d_children;
    RuntimeState *d_state;
    uint32_t *d_root_best;
    uint64_t *d_ht;
    uint32_t *h_root_best;
    RuntimeState *h_state;
    cudaStream_t stream;
} CudaCtx;

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1000.0 + (double)ts.tv_nsec * 1e-6;
}

static void ctx_init(CudaCtx *c) {
    cudaDeviceProp prop;
    CUDA_CHECK(cudaGetDeviceProperties(&prop, 0));
    fprintf(stderr, "selected: %s (CUDA)\n", prop.name);
    fflush(stderr);
    CUDA_CHECK(cudaFuncSetAttribute(select_kernel, cudaFuncAttributeMaxDynamicSharedMemorySize, (int)(SEL_CAP * 2u * sizeof(uint32_t))));
    CUDA_CHECK(cudaStreamCreate(&c->stream));
    CUDA_CHECK(cudaMalloc(&c->d_params, sizeof(SearchParams)));
    CUDA_CHECK(cudaMalloc(&c->d_beam_a, (size_t)MAX_WIDTH * sizeof(GpuNode)));
    CUDA_CHECK(cudaMalloc(&c->d_beam_b, (size_t)MAX_WIDTH * sizeof(GpuNode)));
    CUDA_CHECK(cudaMalloc(&c->d_children, (size_t)MAX_CHILDREN * sizeof(GpuNode)));
    CUDA_CHECK(cudaMalloc(&c->d_state, sizeof(RuntimeState)));
    CUDA_CHECK(cudaMalloc(&c->d_root_best, (size_t)MAX_ROOTS * sizeof(uint32_t)));
    CUDA_CHECK(cudaMalloc(&c->d_ht, (size_t)HT_SIZE * sizeof(uint64_t)));
    CUDA_CHECK(cudaMallocHost(&c->h_root_best, (size_t)MAX_ROOTS * sizeof(uint32_t)));
    CUDA_CHECK(cudaMallocHost(&c->h_state, sizeof(RuntimeState)));
}

static void run_search(CudaCtx *c, const SearchParams *params, const GpuNode *init_beam, uint32_t init_count, float *out_root_best, uint32_t *out_overflow, double *out_ms) {
    double t0 = now_ms();
    if (init_count > MAX_WIDTH) init_count = MAX_WIDTH;
    uint32_t root_count = params->root_count <= MAX_ROOTS ? params->root_count : MAX_ROOTS;
    uint32_t neg_inf = float_to_sortable(-1e30f);
    for (uint32_t i = 0; i < root_count; i++) c->h_root_best[i] = neg_inf;

    RuntimeState st = { 0, init_count, 1, 0, init_count * 2u, 1, 1, 0 };
    *c->h_state = st;

    CUDA_CHECK(cudaMemcpyAsync(c->d_params, params, sizeof(SearchParams), cudaMemcpyHostToDevice, c->stream));
    if (init_count > 0u) {
        CUDA_CHECK(cudaMemcpyAsync(c->d_beam_a, init_beam, (size_t)init_count * sizeof(GpuNode), cudaMemcpyHostToDevice, c->stream));
    }
    CUDA_CHECK(cudaMemcpyAsync(c->d_root_best, c->h_root_best, (size_t)root_count * sizeof(uint32_t), cudaMemcpyHostToDevice, c->stream));
    CUDA_CHECK(cudaMemcpyAsync(c->d_state, c->h_state, sizeof(RuntimeState), cudaMemcpyHostToDevice, c->stream));

    uint32_t plies = (params->depth > 1u) ? (params->depth - 1u) : 0u;
    uint32_t max_blocks = (params->width < MAX_WIDTH ? params->width : MAX_WIDTH) * 2u;
    for (uint32_t p = 0; p < plies && init_count > 0u; p++) {
        uint32_t grid = (p == 0u) ? (init_count * 2u) : max_blocks;
        GpuNode *src = (p & 1u) ? c->d_beam_b : c->d_beam_a;
        GpuNode *dst = (p & 1u) ? c->d_beam_a : c->d_beam_b;
        expand_kernel<<<grid, 64, 0, c->stream>>>(c->d_params, src, c->d_children, c->d_state, c->d_root_best);
        select_kernel<<<1, 256, SEL_CAP * 2u * sizeof(uint32_t), c->stream>>>(c->d_params, c->d_children, dst, c->d_state, c->d_root_best, c->d_ht);
    }

    CUDA_CHECK(cudaMemcpyAsync(c->h_root_best, c->d_root_best, (size_t)root_count * sizeof(uint32_t), cudaMemcpyDeviceToHost, c->stream));
    CUDA_CHECK(cudaMemcpyAsync(c->h_state, c->d_state, sizeof(RuntimeState), cudaMemcpyDeviceToHost, c->stream));
    CUDA_CHECK(cudaStreamSynchronize(c->stream));

    for (uint32_t i = 0; i < root_count; i++) out_root_best[i] = sortable_to_float(c->h_root_best[i]);
    *out_overflow = c->h_state->overflow;
    *out_ms = now_ms() - t0;
}

static int read_exact(void *dst, size_t n) {
    uint8_t *p = (uint8_t *)dst;
    size_t got = 0;
    while (got < n) {
        size_t r = fread(p + got, 1, n - got, stdin);
        if (r == 0) return 0;
        got += r;
    }
    return 1;
}

static int cmd_bench(CudaCtx *c, uint32_t width, uint32_t depth, uint32_t iters) {
    SearchParams p;
    memset(&p, 0, sizeof(p));
    uint32_t bag[14] = {2, 0, 1, 3, 4, 5, 6, 2, 5, 6, 0, 3, 4, 1};
    for (int i = 0; i < 14; i++) p.seq[i] = bag[i];
    p.seq_len = 14; p.depth = depth; p.width = width; p.projection_mode = 2; p.root_count = 32;
    for (int i = 0; i < 24; i++) p.locks[i].multiplier = 1.0f;
    p.w.max_height = -3.0f; p.w.danger_height = -12.0f; p.w.quad_ready = 30.0f; p.w.defense = 12.0f;
    p.w.risk_base = 8.0f; p.w.risk_span = 8.0f; p.w.risk_floor = 0.1f; p.w.combo_state = 14.0f;
    p.w.combo_potential = 6.0f; p.w.race = 1.0f; p.w.holes = -20.0f; p.w.hole_cover = -6.0f;
    p.w.hole_rows = -16.0f; p.w.bumpiness = -3.0f; p.w.bumpiness_sq = -1.0f; p.w.row_transitions = -3.0f;
    p.w.well_depth = 8.0f; p.w.extra_wells = -14.0f; p.w.t_slot1 = 20.0f; p.w.t_slot2 = 80.0f;
    p.w.danger_projected = -12.0f; p.w.stack_exposure = 8.0f; p.w.garbage_rows = -6.0f; p.w.attack = 28.0f;
    p.w.perfect_clear = 120.0f; p.w.b2b_break = -48.0f; p.w.b2b_stock = 14.0f; p.w.wasted_t = -24.0f;
    p.w.surge_charge = 22.0f; p.w.surge_exponent = 1.0f; p.w.tsd_bonus = 20.0f; p.w.combo = 3.0f;
    p.w.opener_attack = 10.0f; p.w.garbage_clear = 18.0f; p.w.survival_flag = 1.0f; p.w.survival_line = 12.0f;
    p.w.incoming = 6.0f; p.w.discount = 0.97f;
    p.w.garbage_cover = -10.0f; p.w.center_height = -2.5f; p.w.col_transitions = -2.0f;
    p.w.t_slot_open_ratio = 0.35f; p.w.cancel_bonus = 12.0f; p.w.hold_t_bonus = 12.0f;
    p.w.hold_i_bonus = 8.0f; p.w.height_mult_scale = 0.45f; p.w.surge_cashout_bonus = 6.0f;
    p.w.well_center_penalty = -6.0f; p.w.downstack_urgency = -1.5f; p.w.b2b_keep_mult = 0.35f;

    GpuNode init[32];
    memset(init, 0, sizeof(init));
    for (uint32_t i = 0; i < 32; i++) {
        for (int y = 0; y < 6; y++) {
            uint32_t r = 0x3FFu & ~(1u << ((y * 3 + 1) % 10));
            init[i].rows[y >> 1] |= r << ((y & 1) * 16);
        }
        init[i].meta0 = 1u | (6u << 8u);
        init[i].meta1 = (i << 16u) | (2u << 24u);
    }

    float rb[MAX_ROOTS];
    uint32_t ov = 0, total_ov = 0;
    double ms = 0.0, sum = 0.0, best = 1e9, worst = 0.0;
    run_search(c, &p, init, 32, rb, &ov, &ms);
    for (uint32_t it = 0; it < iters; it++) {
        run_search(c, &p, init, 32, rb, &ov, &ms);
        sum += ms; if (ms < best) best = ms; if (ms > worst) worst = ms; total_ov += ov;
    }
    float bv = -1e30f;
    for (uint32_t i = 0; i < 32; i++) if (rb[i] > bv) bv = rb[i];
    printf("CUDA d%u w%u (%u iters): mean=%.2fms min=%.2fms max=%.2fms (%.1f dec/s) best_val=%.2f overflow=%u\n",
           depth, width, iters, sum / iters, best, worst, 1000.0 / (sum / iters), bv, total_ov);
    return 0;
}

static int cmd_serve(CudaCtx *c) {
    GpuNode *init_beam = (GpuNode *)calloc(MAX_WIDTH, sizeof(GpuNode));
    float root_best[MAX_ROOTS];
    for (;;) {
        uint32_t magic = 0;
        if (!read_exact(&magic, 4)) break;
        if (magic != PROTO_MAGIC) { fprintf(stderr, "serve: bad magic 0x%08x\n", magic); free(init_beam); return 1; }
        SearchParams params;
        uint32_t init_count = 0;
        if (!read_exact(&params, sizeof(params))) break;
        if (!read_exact(&init_count, 4)) break;
        if (init_count > MAX_WIDTH) { fprintf(stderr, "serve: init_count %u > MAX_WIDTH\n", init_count); free(init_beam); return 1; }
        if (init_count > 0 && !read_exact(init_beam, (size_t)init_count * sizeof(GpuNode))) break;

        uint32_t ov = 0;
        double ms = 0.0;
        run_search(c, &params, init_beam, init_count, root_best, &ov, &ms);

        uint32_t rc = params.root_count <= MAX_ROOTS ? params.root_count : MAX_ROOTS;
        uint32_t hdr[3] = { PROTO_MAGIC, rc, ov };
        float wall_ms = (float)ms;
        fwrite(hdr, 4, 3, stdout);
        fwrite(&wall_ms, 4, 1, stdout);
        fwrite(root_best, 4, rc, stdout);
        fflush(stdout);
    }
    free(init_beam);
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: %s <unused> <unused> <bench [w] [d] [iters] | serve>\n", argv[0]);
        return 1;
    }
    CudaCtx c;
    ctx_init(&c);
    if (strcmp(argv[3], "bench") == 0) {
        uint32_t w = argc > 4 ? (uint32_t)atoi(argv[4]) : 256u;
        uint32_t d = argc > 5 ? (uint32_t)atoi(argv[5]) : 14u;
        uint32_t it = argc > 6 ? (uint32_t)atoi(argv[6]) : 30u;
        return cmd_bench(&c, w, d, it);
    } else if (strcmp(argv[3], "serve") == 0) {
        return cmd_serve(&c);
    }
    fprintf(stderr, "unknown mode: %s\n", argv[3]);
    return 1;
}
