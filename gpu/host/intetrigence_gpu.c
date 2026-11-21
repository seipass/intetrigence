// Intetrigence Vulkan 1.2 Compute Beam Search Host.
// Drives shaders/expand.spv + shaders/select.spv in a single batched command
// buffer per decision with proper COMPUTE_SHADER | DRAW_INDIRECT barriers.

#define _POSIX_C_SOURCE 200809L
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <vulkan/vulkan.h>

#define MAX_WIDTH    2048u
#define MAX_CHILDREN 262144u
#define MAX_ROOTS    256u
#define HT_SIZE      4096u
#define PROTO_MAGIC  0x49475055u

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

_Static_assert(sizeof(GpuNode) == 128, "GpuNode size mismatch");
_Static_assert(sizeof(EvalWeights) == 288, "EvalWeights size mismatch");
_Static_assert(sizeof(SearchParams) == 800, "SearchParams size mismatch");
_Static_assert(sizeof(RuntimeState) == 32, "RuntimeState size mismatch");

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1000.0 + (double)ts.tv_nsec * 1e-6;
}

static uint32_t float_to_sortable(float f) {
    uint32_t u;
    memcpy(&u, &f, 4);
    return (u & 0x80000000u) ? ~u : (u | 0x80000000u);
}

static float sortable_to_float(uint32_t s) {
    uint32_t u = (s & 0x80000000u) ? (s & 0x7FFFFFFFu) : ~s;
    float f;
    memcpy(&f, &u, 4);
    return f;
}

#define VK_CHECK(x) do { \
    VkResult _r = (x); \
    if (_r != VK_SUCCESS) { \
        fprintf(stderr, "Vulkan error %d at %s:%d\n", (int)_r, __FILE__, __LINE__); \
        exit(1); \
    } \
} while (0)

typedef struct {
    VkBuffer buf;
    VkDeviceMemory mem;
    void *mapped;
    VkDeviceSize size;
} Buffer;

typedef struct {
    VkInstance instance;
    VkPhysicalDevice phys;
    VkDevice dev;
    VkQueue queue;
    uint32_t queue_family;
    VkCommandPool cmd_pool;
    VkCommandBuffer cmd;
    VkFence fence;
    VkDescriptorSetLayout dsl;
    VkPipelineLayout pl;
    VkPipeline pipe_expand;
    VkPipeline pipe_select;
    VkDescriptorPool dpool;
    VkDescriptorSet ds_expand_ab;
    VkDescriptorSet ds_expand_ba;
    VkDescriptorSet ds_select_ab;
    VkDescriptorSet ds_select_ba;
    Buffer params_buf;
    Buffer beam_a;
    Buffer beam_b;
    Buffer children_buf;
    Buffer state_buf;
    Buffer root_best_buf;
    Buffer ht_buf;
    char device_name[256];
} GpuCtx;

static uint32_t find_mem_type(VkPhysicalDevice phys, uint32_t bits, VkMemoryPropertyFlags want) {
    VkPhysicalDeviceMemoryProperties mp;
    vkGetPhysicalDeviceMemoryProperties(phys, &mp);
    for (uint32_t i = 0; i < mp.memoryTypeCount; i++) {
        if ((bits & (1u << i)) && ((mp.memoryTypes[i].propertyFlags & want) == want)) return i;
    }
    fprintf(stderr, "No matching Vulkan memory type (want=0x%x)\n", (unsigned)want);
    exit(1);
}

static Buffer make_buf(GpuCtx *ctx, VkDeviceSize size, VkBufferUsageFlags usage, VkMemoryPropertyFlags props, int map) {
    Buffer b = {0};
    b.size = size;
    VkBufferCreateInfo bci = {
        .sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
        .size = size,
        .usage = usage,
        .sharingMode = VK_SHARING_MODE_EXCLUSIVE,
    };
    VK_CHECK(vkCreateBuffer(ctx->dev, &bci, NULL, &b.buf));
    VkMemoryRequirements mr;
    vkGetBufferMemoryRequirements(ctx->dev, b.buf, &mr);
    VkMemoryAllocateInfo mai = {
        .sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
        .allocationSize = mr.size,
        .memoryTypeIndex = find_mem_type(ctx->phys, mr.memoryTypeBits, props),
    };
    VK_CHECK(vkAllocateMemory(ctx->dev, &mai, NULL, &b.mem));
    VK_CHECK(vkBindBufferMemory(ctx->dev, b.buf, b.mem, 0));
    if (map) {
        VK_CHECK(vkMapMemory(ctx->dev, b.mem, 0, size, 0, &b.mapped));
    }
    return b;
}

static VkShaderModule load_spv(VkDevice dev, const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) { fprintf(stderr, "Cannot open shader %s\n", path); exit(1); }
    fseek(f, 0, SEEK_END);
    long len = ftell(f);
    fseek(f, 0, SEEK_SET);
    uint32_t *code = malloc((size_t)len);
    if (!code || fread(code, 1, (size_t)len, f) != (size_t)len) {
        fprintf(stderr, "Failed to read shader %s\n", path);
        exit(1);
    }
    fclose(f);
    VkShaderModuleCreateInfo ci = {
        .sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
        .codeSize = (size_t)len,
        .pCode = code,
    };
    VkShaderModule mod;
    VK_CHECK(vkCreateShaderModule(dev, &ci, NULL, &mod));
    free(code);
    return mod;
}

static void bind_ds(VkDevice dev, VkDescriptorSet ds, VkBuffer b0, VkBuffer b1, VkBuffer b2, VkBuffer b3, VkBuffer b4, VkBuffer b5) {
    VkDescriptorBufferInfo bi[6] = {
        { b0, 0, VK_WHOLE_SIZE },
        { b1, 0, VK_WHOLE_SIZE },
        { b2, 0, VK_WHOLE_SIZE },
        { b3, 0, VK_WHOLE_SIZE },
        { b4, 0, VK_WHOLE_SIZE },
        { b5, 0, VK_WHOLE_SIZE },
    };
    VkWriteDescriptorSet w[6];
    for (int i = 0; i < 6; i++) {
        w[i] = (VkWriteDescriptorSet){
            .sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
            .dstSet = ds,
            .dstBinding = (uint32_t)i,
            .descriptorCount = 1,
            .descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
            .pBufferInfo = &bi[i],
        };
    }
    vkUpdateDescriptorSets(dev, 6, w, 0, NULL);
}

static void gpu_init(GpuCtx *ctx, const char *expand_spv, const char *select_spv) {
    memset(ctx, 0, sizeof(*ctx));
    VkApplicationInfo app = {
        .sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
        .pApplicationName = "intetrigence-gpu",
        .apiVersion = VK_API_VERSION_1_2,
    };
    VkInstanceCreateInfo ici = {
        .sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
        .pApplicationInfo = &app,
    };
    VK_CHECK(vkCreateInstance(&ici, NULL, &ctx->instance));

    uint32_t ndev = 0;
    VK_CHECK(vkEnumeratePhysicalDevices(ctx->instance, &ndev, NULL));
    if (ndev == 0) { fprintf(stderr, "No Vulkan devices found\n"); exit(1); }
    VkPhysicalDevice *devs = calloc(ndev, sizeof(*devs));
    VK_CHECK(vkEnumeratePhysicalDevices(ctx->instance, &ndev, devs));

    const char *want = getenv("INTETRIGENCE_GPU_DEVICE");
    int chosen = -1;
    for (uint32_t i = 0; i < ndev; i++) {
        VkPhysicalDeviceProperties p;
        vkGetPhysicalDeviceProperties(devs[i], &p);
        if (want && want[0] != '\0') {
            if (strstr(p.deviceName, want)) { chosen = (int)i; break; }
        } else {
            if (p.deviceType == VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU) { chosen = (int)i; break; }
            if (chosen < 0 && p.deviceType != VK_PHYSICAL_DEVICE_TYPE_CPU) chosen = (int)i;
        }
    }
    if (chosen < 0) chosen = 0;
    ctx->phys = devs[chosen];
    VkPhysicalDeviceProperties props;
    vkGetPhysicalDeviceProperties(ctx->phys, &props);
    snprintf(ctx->device_name, sizeof(ctx->device_name), "%s", props.deviceName);
    fprintf(stderr, "selected: %s\n", ctx->device_name);
    fflush(stderr);
    free(devs);

    uint32_t nqf = 0;
    vkGetPhysicalDeviceQueueFamilyProperties(ctx->phys, &nqf, NULL);
    VkQueueFamilyProperties *qfp = calloc(nqf, sizeof(*qfp));
    vkGetPhysicalDeviceQueueFamilyProperties(ctx->phys, &nqf, qfp);
    ctx->queue_family = UINT32_MAX;
    for (uint32_t i = 0; i < nqf; i++) {
        if (qfp[i].queueFlags & VK_QUEUE_COMPUTE_BIT) { ctx->queue_family = i; break; }
    }
    free(qfp);
    if (ctx->queue_family == UINT32_MAX) { fprintf(stderr, "No compute queue family\n"); exit(1); }

    float prio = 1.0f;
    VkDeviceQueueCreateInfo qci = {
        .sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
        .queueFamilyIndex = ctx->queue_family,
        .queueCount = 1,
        .pQueuePriorities = &prio,
    };
    VkDeviceCreateInfo dci = {
        .sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
        .queueCreateInfoCount = 1,
        .pQueueCreateInfos = &qci,
    };
    VK_CHECK(vkCreateDevice(ctx->phys, &dci, NULL, &ctx->dev));
    vkGetDeviceQueue(ctx->dev, ctx->queue_family, 0, &ctx->queue);

    VkCommandPoolCreateInfo cpci = {
        .sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
        .flags = VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
        .queueFamilyIndex = ctx->queue_family,
    };
    VK_CHECK(vkCreateCommandPool(ctx->dev, &cpci, NULL, &ctx->cmd_pool));
    VkCommandBufferAllocateInfo cbai = {
        .sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
        .commandPool = ctx->cmd_pool,
        .level = VK_COMMAND_BUFFER_LEVEL_PRIMARY,
        .commandBufferCount = 1,
    };
    VK_CHECK(vkAllocateCommandBuffers(ctx->dev, &cbai, &ctx->cmd));
    VkFenceCreateInfo fci = { .sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO };
    VK_CHECK(vkCreateFence(ctx->dev, &fci, NULL, &ctx->fence));

    // Descriptor layout: 6 storage buffers
    VkDescriptorSetLayoutBinding bindings[6];
    for (uint32_t i = 0; i < 6; i++) {
        bindings[i] = (VkDescriptorSetLayoutBinding){
            .binding = i,
            .descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
            .descriptorCount = 1,
            .stageFlags = VK_SHADER_STAGE_COMPUTE_BIT,
        };
    }
    VkDescriptorSetLayoutCreateInfo dslci = {
        .sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
        .bindingCount = 6,
        .pBindings = bindings,
    };
    VK_CHECK(vkCreateDescriptorSetLayout(ctx->dev, &dslci, NULL, &ctx->dsl));
    VkPipelineLayoutCreateInfo plci = {
        .sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
        .setLayoutCount = 1,
        .pSetLayouts = &ctx->dsl,
    };
    VK_CHECK(vkCreatePipelineLayout(ctx->dev, &plci, NULL, &ctx->pl));

    VkShaderModule mod_ex = load_spv(ctx->dev, expand_spv);
    VkShaderModule mod_se = load_spv(ctx->dev, select_spv);
    VkComputePipelineCreateInfo p_ci[2] = {
        {
            .sType = VK_STRUCTURE_TYPE_COMPUTE_PIPELINE_CREATE_INFO,
            .stage = { .sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO, .stage = VK_SHADER_STAGE_COMPUTE_BIT, .module = mod_ex, .pName = "main" },
            .layout = ctx->pl,
        },
        {
            .sType = VK_STRUCTURE_TYPE_COMPUTE_PIPELINE_CREATE_INFO,
            .stage = { .sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO, .stage = VK_SHADER_STAGE_COMPUTE_BIT, .module = mod_se, .pName = "main" },
            .layout = ctx->pl,
        },
    };
    VkPipeline pipes[2];
    VK_CHECK(vkCreateComputePipelines(ctx->dev, VK_NULL_HANDLE, 2, p_ci, NULL, pipes));
    ctx->pipe_expand = pipes[0];
    ctx->pipe_select = pipes[1];
    vkDestroyShaderModule(ctx->dev, mod_ex, NULL);
    vkDestroyShaderModule(ctx->dev, mod_se, NULL);

    VkMemoryPropertyFlags host_vis = VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
    VkMemoryPropertyFlags dev_loc = VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT;

    ctx->params_buf    = make_buf(ctx, sizeof(SearchParams), VK_BUFFER_USAGE_STORAGE_BUFFER_BIT, host_vis, 1);
    ctx->beam_a        = make_buf(ctx, (VkDeviceSize)MAX_WIDTH * sizeof(GpuNode), VK_BUFFER_USAGE_STORAGE_BUFFER_BIT, host_vis, 1);
    ctx->beam_b        = make_buf(ctx, (VkDeviceSize)MAX_WIDTH * sizeof(GpuNode), VK_BUFFER_USAGE_STORAGE_BUFFER_BIT, dev_loc, 0);
    ctx->children_buf  = make_buf(ctx, (VkDeviceSize)MAX_CHILDREN * sizeof(GpuNode), VK_BUFFER_USAGE_STORAGE_BUFFER_BIT, dev_loc, 0);
    ctx->state_buf     = make_buf(ctx, sizeof(RuntimeState), VK_BUFFER_USAGE_STORAGE_BUFFER_BIT | VK_BUFFER_USAGE_INDIRECT_BUFFER_BIT, host_vis, 1);
    ctx->root_best_buf = make_buf(ctx, (VkDeviceSize)MAX_ROOTS * sizeof(uint32_t), VK_BUFFER_USAGE_STORAGE_BUFFER_BIT, host_vis, 1);
    ctx->ht_buf        = make_buf(ctx, (VkDeviceSize)HT_SIZE * 2u * sizeof(uint32_t), VK_BUFFER_USAGE_STORAGE_BUFFER_BIT, dev_loc, 0);

    VkDescriptorPoolSize dps = { .type = VK_DESCRIPTOR_TYPE_STORAGE_BUFFER, .descriptorCount = 24 };
    VkDescriptorPoolCreateInfo dpci = {
        .sType = VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO,
        .maxSets = 4,
        .poolSizeCount = 1,
        .pPoolSizes = &dps,
    };
    VK_CHECK(vkCreateDescriptorPool(ctx->dev, &dpci, NULL, &ctx->dpool));
    VkDescriptorSetLayout layouts[4] = { ctx->dsl, ctx->dsl, ctx->dsl, ctx->dsl };
    VkDescriptorSet sets[4];
    VkDescriptorSetAllocateInfo dsai = {
        .sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO,
        .descriptorPool = ctx->dpool,
        .descriptorSetCount = 4,
        .pSetLayouts = layouts,
    };
    VK_CHECK(vkAllocateDescriptorSets(ctx->dev, &dsai, sets));
    ctx->ds_expand_ab = sets[0];
    ctx->ds_expand_ba = sets[1];
    ctx->ds_select_ab = sets[2];
    ctx->ds_select_ba = sets[3];

    bind_ds(ctx->dev, ctx->ds_expand_ab, ctx->params_buf.buf, ctx->beam_a.buf, ctx->children_buf.buf, ctx->state_buf.buf, ctx->root_best_buf.buf, ctx->ht_buf.buf);
    bind_ds(ctx->dev, ctx->ds_expand_ba, ctx->params_buf.buf, ctx->beam_b.buf, ctx->children_buf.buf, ctx->state_buf.buf, ctx->root_best_buf.buf, ctx->ht_buf.buf);
    bind_ds(ctx->dev, ctx->ds_select_ab, ctx->params_buf.buf, ctx->children_buf.buf, ctx->beam_b.buf, ctx->state_buf.buf, ctx->root_best_buf.buf, ctx->ht_buf.buf);
    bind_ds(ctx->dev, ctx->ds_select_ba, ctx->params_buf.buf, ctx->children_buf.buf, ctx->beam_a.buf, ctx->state_buf.buf, ctx->root_best_buf.buf, ctx->ht_buf.buf);
}

static void gpu_run_search(GpuCtx *ctx, const SearchParams *params, const GpuNode *init_beam, uint32_t init_count, float *out_root_best, uint32_t *out_overflow, double *out_ms) {
    double t0 = now_ms();
    if (init_count > MAX_WIDTH) init_count = MAX_WIDTH;
    uint32_t root_count = params->root_count;
    if (root_count > MAX_ROOTS) root_count = MAX_ROOTS;

    memcpy(ctx->params_buf.mapped, params, sizeof(SearchParams));
    memcpy(ctx->beam_a.mapped, init_beam, (size_t)init_count * sizeof(GpuNode));

    uint32_t *rb = (uint32_t *)ctx->root_best_buf.mapped;
    uint32_t neg_inf = float_to_sortable(-1e30f);
    for (uint32_t i = 0; i < root_count; i++) rb[i] = neg_inf;

    RuntimeState st = {
        .child_count = 0,
        .beam_count = init_count,
        .level = 1, // ply 0 already expanded on CPU!
        .overflow = 0,
        .dispatch_x = init_count * 2u,
        .dispatch_y = 1,
        .dispatch_z = 1,
        .pad0 = 0,
    };
    memcpy(ctx->state_buf.mapped, &st, sizeof(st));

    uint32_t plies = (params->depth > 1u) ? (params->depth - 1u) : 0u;
    if (plies > 0u && init_count > 0u) {
        VK_CHECK(vkResetCommandBuffer(ctx->cmd, 0));
        VkCommandBufferBeginInfo bi = {
            .sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            .flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
        };
        VK_CHECK(vkBeginCommandBuffer(ctx->cmd, &bi));

        VkMemoryBarrier mb_compute = {
            .sType = VK_STRUCTURE_TYPE_MEMORY_BARRIER,
            .srcAccessMask = VK_ACCESS_SHADER_WRITE_BIT,
            .dstAccessMask = VK_ACCESS_SHADER_READ_BIT | VK_ACCESS_SHADER_WRITE_BIT,
        };
        VkMemoryBarrier mb_indirect = {
            .sType = VK_STRUCTURE_TYPE_MEMORY_BARRIER,
            .srcAccessMask = VK_ACCESS_SHADER_WRITE_BIT,
            .dstAccessMask = VK_ACCESS_SHADER_READ_BIT | VK_ACCESS_SHADER_WRITE_BIT | VK_ACCESS_INDIRECT_COMMAND_READ_BIT,
        };

        for (uint32_t p = 0; p < plies; p++) {
            int ping = (int)(p & 1u);
            VkDescriptorSet ds_ex = ping ? ctx->ds_expand_ba : ctx->ds_expand_ab;
            VkDescriptorSet ds_se = ping ? ctx->ds_select_ba : ctx->ds_select_ab;

            vkCmdBindPipeline(ctx->cmd, VK_PIPELINE_BIND_POINT_COMPUTE, ctx->pipe_expand);
            vkCmdBindDescriptorSets(ctx->cmd, VK_PIPELINE_BIND_POINT_COMPUTE, ctx->pl, 0, 1, &ds_ex, 0, NULL);
            vkCmdDispatchIndirect(ctx->cmd, ctx->state_buf.buf, 16);

            vkCmdPipelineBarrier(ctx->cmd,
                VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT,
                VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT,
                0, 1, &mb_compute, 0, NULL, 0, NULL);

            vkCmdBindPipeline(ctx->cmd, VK_PIPELINE_BIND_POINT_COMPUTE, ctx->pipe_select);
            vkCmdBindDescriptorSets(ctx->cmd, VK_PIPELINE_BIND_POINT_COMPUTE, ctx->pl, 0, 1, &ds_se, 0, NULL);
            vkCmdDispatch(ctx->cmd, 1, 1, 1);

            if (p + 1u < plies) {
                vkCmdPipelineBarrier(ctx->cmd,
                    VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT,
                    VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT | VK_PIPELINE_STAGE_DRAW_INDIRECT_BIT,
                    0, 1, &mb_indirect, 0, NULL, 0, NULL);
            }
        }
        VK_CHECK(vkEndCommandBuffer(ctx->cmd));

        VK_CHECK(vkResetFences(ctx->dev, 1, &ctx->fence));
        VkSubmitInfo si = {
            .sType = VK_STRUCTURE_TYPE_SUBMIT_INFO,
            .commandBufferCount = 1,
            .pCommandBuffers = &ctx->cmd,
        };
        VK_CHECK(vkQueueSubmit(ctx->queue, 1, &si, ctx->fence));
        VK_CHECK(vkWaitForFences(ctx->dev, 1, &ctx->fence, VK_TRUE, UINT64_MAX));
    }

    RuntimeState out_st;
    memcpy(&out_st, ctx->state_buf.mapped, sizeof(out_st));
    for (uint32_t i = 0; i < root_count; i++) {
        out_root_best[i] = sortable_to_float(rb[i]);
    }
    *out_overflow = out_st.overflow;
    *out_ms = now_ms() - t0;
}

static EvalWeights default_profile6_weights(void) {
    EvalWeights w = {
        .max_height = -3.0f, .height_sq = 0.0f, .danger_height = -12.0f,
        .quad_ready = 30.0f, .defense = 12.0f,
        .risk_base = 8.0f, .risk_span = 8.0f, .risk_floor = 0.1f,
        .opener_clear = 0.0f, .combo_state = 14.0f, .combo_potential = 6.0f, .race = 1.0f,
        .holes = -20.0f, .hole_cover = -6.0f, .hole_rows = -16.0f,
        .bumpiness = -3.0f, .bumpiness_sq = -1.0f, .row_transitions = -3.0f,
        .well_depth = 8.0f, .extra_wells = -14.0f,
        .t_slot0 = 0.0f, .t_slot1 = 20.0f, .t_slot2 = 80.0f,
        .danger_projected = -12.0f, .stack_exposure = 8.0f, .garbage_rows = -6.0f,
        .attack = 28.0f, .root_sent = 0.0f,
        .clear_w = {
            0.0f, -18.0f, -12.0f, -8.0f, 18.0f,
            0.0f,   4.0f,  14.0f, 24.0f, 36.0f,
            0.0f,  20.0f,  40.0f, 50.0f, 60.0f,
        },
        .perfect_clear = 120.0f, .b2b_break = -48.0f, .b2b_stock = 14.0f, .margin_stock = 0.0f,
        .wasted_t = -24.0f, .surge_charge = 22.0f, .surge_exponent = 1.0f, .surge_hold_penalty = 0.0f,
        .tsd_bonus = 20.0f, .mini_penalty = 0.0f, .combo = 3.0f, .opener_attack = 10.0f,
        .garbage_clear = 18.0f, .survival_flag = 1.0f, .survival_line = 12.0f,
        .incoming = 6.0f, .discount = 0.97f,
        .garbage_cover = -10.0f, .center_height = -2.5f, .col_transitions = -2.0f,
        .t_slot_open_ratio = 0.35f, .cancel_bonus = 12.0f, .hold_t_bonus = 12.0f,
        .hold_i_bonus = 8.0f, .height_mult_scale = 0.45f, .surge_cashout_bonus = 6.0f,
        .well_center_penalty = -6.0f, .downstack_urgency = -1.5f, .b2b_keep_mult = 0.35f,
    };
    return w;
}

static int cmd_bench(GpuCtx *ctx, uint32_t width, uint32_t depth, uint32_t iters) {
    SearchParams p;
    memset(&p, 0, sizeof(p));
    uint32_t bag[14] = {2, 0, 1, 3, 4, 5, 6, 2, 5, 6, 0, 3, 4, 1};
    for (int i = 0; i < 14; i++) p.seq[i] = bag[i];
    p.seq_len = 14;
    p.depth = depth;
    p.width = width;
    p.projection_mode = 2;
    p.root_count = 32;
    for (int i = 0; i < 24; i++) p.locks[i].multiplier = 1.0f;
    p.w = default_profile6_weights();

    GpuNode init[32];
    memset(init, 0, sizeof(init));
    for (uint32_t i = 0; i < 32; i++) {
        // 6-row mid-game board with hole in col 9
        for (int y = 0; y < 6; y++) {
            uint32_t r = 0x3FFu & ~(1u << ((y * 3 + 1) % 10));
            init[i].rows[y >> 1] |= r << ((y & 1) * 16);
        }
        init[i].meta0 = 1u | (6u << 8u); // next=1, hold=L(6)
        init[i].meta1 = (i << 16u) | (2u << 24u); // root_idx=i, can_hold=1
    }

    float rb[MAX_ROOTS];
    uint32_t ov = 0;
    double ms = 0.0;
    // Warmup
    gpu_run_search(ctx, &p, init, 32, rb, &ov, &ms);

    double sum = 0.0, best = 1e9, worst = 0.0;
    uint32_t total_ov = 0;
    for (uint32_t it = 0; it < iters; it++) {
        gpu_run_search(ctx, &p, init, 32, rb, &ov, &ms);
        sum += ms;
        if (ms < best) best = ms;
        if (ms > worst) worst = ms;
        total_ov += ov;
    }
    float best_v = -1e30f;
    for (uint32_t i = 0; i < 32; i++) if (rb[i] > best_v) best_v = rb[i];
    printf("d%u w%u (%u iters): mean=%.2fms min=%.2fms max=%.2fms (%.1f dec/s) best_val=%.2f overflow=%u\n",
           depth, width, iters, sum / iters, best, worst, 1000.0 / (sum / iters), best_v, total_ov);
    return 0;
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

static int cmd_serve(GpuCtx *ctx) {
    GpuNode *init_beam = calloc(MAX_WIDTH, sizeof(GpuNode));
    float root_best[MAX_ROOTS];
    for (;;) {
        uint32_t magic = 0;
        if (!read_exact(&magic, 4)) break;
        if (magic != PROTO_MAGIC) {
            fprintf(stderr, "serve: bad magic 0x%08x\n", magic);
            free(init_beam);
            return 1;
        }
        SearchParams params;
        uint32_t init_count = 0;
        if (!read_exact(&params, sizeof(params))) break;
        if (!read_exact(&init_count, 4)) break;
        if (init_count > MAX_WIDTH) {
            fprintf(stderr, "serve: init_count %u > MAX_WIDTH\n", init_count);
            free(init_beam);
            return 1;
        }
        if (init_count > 0 && !read_exact(init_beam, (size_t)init_count * sizeof(GpuNode))) break;

        uint32_t ov = 0;
        double ms = 0.0;
        gpu_run_search(ctx, &params, init_beam, init_count, root_best, &ov, &ms);

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
        fprintf(stderr, "usage: %s <expand.spv> <select.spv> <bench [w] [d] [iters] | serve>\n", argv[0]);
        return 1;
    }
    GpuCtx ctx;
    gpu_init(&ctx, argv[1], argv[2]);
    if (strcmp(argv[3], "bench") == 0) {
        uint32_t w = argc > 4 ? (uint32_t)atoi(argv[4]) : 256u;
        uint32_t d = argc > 5 ? (uint32_t)atoi(argv[5]) : 14u;
        uint32_t it = argc > 6 ? (uint32_t)atoi(argv[6]) : 30u;
        return cmd_bench(&ctx, w, d, it);
    } else if (strcmp(argv[3], "serve") == 0) {
        return cmd_serve(&ctx);
    }
    fprintf(stderr, "unknown mode: %s\n", argv[3]);
    return 1;
}
