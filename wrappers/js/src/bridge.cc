#if __has_include(<napi.h>)
#include <napi.h>
#else
#include "node-addon-api/napi.h"
#endif

#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <cmath>
#include <string>
#include <vector>
#include <atomic>
#include <memory>
#include <thread>
#include "include/quantion.h"

#if defined(_WIN32)
#include <windows.h>
#define DLIB HMODULE
#define DLOPEN(p) LoadLibraryA(p)
#define DLSYM(h, s) GetProcAddress(h, s)
#define DLCLOSE(h) FreeLibrary(h)
static const char *last_err = "LoadLibrary/GetProcAddress failed";
#else
#include <dlfcn.h>
#define DLIB void *
#define DLOPEN(p) dlopen(p, RTLD_NOW | RTLD_GLOBAL)
#define DLSYM(h, s) dlsym(h, s)
#define DLCLOSE(h) dlclose(h)
static const char *last_err = NULL;
#endif

typedef struct MzML MzML;
typedef struct Model Model;
typedef struct WarpModel WarpModel;

struct RangeStore
{
  Napi::Env env;
  Napi::FunctionReference read;
  std::thread::id owner;
};

struct MzMLWrapper
{
  MzML *ptr;
  size_t estimated_size;
  RangeStore *store;
};

static int32_t serve_range(void *context, uint64_t offset, uint64_t length, uint8_t *dest)
{
  RangeStore *store = (RangeStore *)context;
  if (store == nullptr || store->read.IsEmpty())
    return -1;
  if (std::this_thread::get_id() != store->owner)
    return -1;

  Napi::Env env = store->env;
  Napi::HandleScope scope(env);
  Napi::Value answer;
  try
  {
    answer = store->read.Call({
        Napi::Number::New(env, (double)offset),
        Napi::Number::New(env, (double)length),
    });
  }
  catch (const Napi::Error &)
  {
    return -1;
  }

  if (!answer.IsBuffer())
    return -1;
  Napi::Buffer<uint8_t> bytes = answer.As<Napi::Buffer<uint8_t>>();
  if (bytes.Length() != (size_t)length)
    return -1;
  std::memcpy(dest, bytes.Data(), (size_t)length);
  return 0;
}

static_assert(sizeof(CPeakOptions) == 80, "CPeakOptions must be 80 bytes");

typedef uint32_t (*fn_quantion_abi_version)(void);
typedef size_t   (*fn_quantion_sizeof_peak_options)(void);
typedef int32_t (*fn_parse_mzml)(const unsigned char *, size_t, MzML **);
typedef int32_t (*fn_parse_bin)(const unsigned char *, size_t, size_t, MzML **);
typedef int32_t (*fn_read_range)(void *, uint64_t, uint64_t, uint8_t *);
typedef int32_t (*fn_parse_ion_source)(fn_read_range, void *, size_t, MzML **);
typedef int32_t (*fn_plan_open)(const uint8_t *, size_t, Buf *);
typedef int32_t (*fn_plan_eic)(MzML *, double, double, double, double, double, uint8_t, Buf *);
typedef int32_t (*fn_plan_find_targets)(MzML *, const double *, size_t, double, double, double, double, uint8_t, Buf *);
typedef int32_t (*fn_plan_find_features)(MzML *, double, double, double, double, uint8_t, double, double, double, Buf *);
typedef int32_t (*fn_plan_scans)(MzML *, uint8_t, double, double, uint8_t, uint8_t, Buf *);
typedef int32_t (*fn_scans_with_arrays)(MzML *, uint8_t, double, double, uint8_t, uint8_t, const uint32_t *, size_t, Buf *);
typedef int32_t (*fn_plan_image)(MzML *, double, double, uint8_t, Buf *);
typedef int32_t (*fn_parse_ion_path)(const char *, size_t, MzML **);
typedef void (*fn_free_mzml)(MzML *);
typedef int32_t (*fn_quantion_last_error)(Buf *);
typedef int32_t (*fn_parse_mzml_path)(const char *, MzML **);
typedef int32_t (*fn_write_ion)(MzML *, const char *, uint8_t, uint8_t, uint8_t, uint32_t, uint32_t, const uint8_t *, size_t, Buf *);
typedef int32_t (*fn_write_mzml)(MzML *, const char *, uint8_t, Buf *);
typedef int32_t (*fn_fit_peak)(const double *, const double *, size_t, double, int32_t, double, Buf *);
typedef int32_t (*fn_draw_peaks)(const double *, const double *, const double *, const double *, size_t, double, double, size_t, Buf *, Buf *);
typedef int32_t (*fn_get_peak)(const double *, const double *, size_t, double, double, const CPeakOptions *, Buf *);
typedef int32_t (*fn_calculate_eic)(const MzML *, double, double, double, double, double, uint8_t, Buf *, Buf *);
typedef int32_t (*fn_find_noise_level)(const float *, size_t, size_t *, double *);
typedef int32_t (*fn_find_targets)(const MzML *, const double *, const double *, const double *, const uint32_t *, const uint32_t *, const unsigned char *, size_t, size_t, double, double, double, double, uint8_t, const CPeakOptions *, size_t, Buf *);
typedef int32_t (*fn_find_transitions)(const MzML *, const double *, const double *, const double *, const double *, const uint32_t *, const uint32_t *, const uint8_t *, size_t, size_t, const CPeakOptions *, size_t, Buf *);
typedef int32_t (*fn_find_peaks)(const double *, const double *, size_t, const CPeakOptions *, Buf *);
typedef int32_t (*fn_calculate_baseline)(const double *, size_t, double, size_t, Buf *);
typedef int32_t (*fn_deisotope_features)(const double *, size_t, double, double, Buf *);
typedef int32_t (*fn_find_features)(const MzML *, double, double, double, double, uint8_t, double, double, double, const CPeakOptions *, size_t, Buf *);
typedef int32_t (*fn_match_features)(const char *, double, double, double, double, uint8_t, double, double, double, double, double, double, double, const CPeakOptions *, size_t, Buf *);
typedef int32_t (*fn_get_scans)(const MzML *, uint8_t, double, double, uint8_t, uint8_t, Buf *);
typedef int32_t (*fn_scan_metadata)(MzML *, size_t, const char *, Buf *);
typedef int32_t (*fn_plan_scan_metadata)(MzML *, size_t, Buf *);
typedef int32_t (*fn_get_ion_image)(const MzML *, double, double, uint8_t, Buf *);
typedef void (*fn_free_)(unsigned char *, size_t);
typedef int32_t (*fn_model_load)(const uint8_t *, size_t, Model **, Buf *);
typedef void (*fn_model_free)(Model *);
typedef int32_t (*fn_model_metadata)(const Model *, Buf *);
typedef int32_t (*fn_warp_model_load)(const Model *, WarpModel **, Buf *);
typedef void (*fn_warp_model_free)(WarpModel *);
typedef int32_t (*fn_calibrate_rts)(const WarpModel *, const MzML *const *, size_t, const size_t *, size_t,
                                    const uint32_t *, const uint32_t *, const unsigned char *, size_t,
                                    const double *, const double *, size_t, Buf *);
typedef int32_t (*fn_predict_rts_from_file)(const WarpModel *, const MzML *, const double *, const double *, size_t, Buf *);
typedef int32_t (*fn_plan_predict_rts)(const WarpModel *, MzML *, const double *, const double *, size_t, Buf *);

typedef struct
{
  fn_quantion_abi_version quantion_abi_version;
  fn_quantion_sizeof_peak_options quantion_sizeof_peak_options;
  fn_parse_mzml parse_mzml;
  fn_quantion_last_error quantion_last_error;
  fn_parse_mzml_path parse_mzml_path;
  fn_write_ion write_ion;
  fn_write_mzml write_mzml;
  fn_fit_peak fit_peak;
  fn_draw_peaks draw_peaks;
  fn_get_peak get_peak;
  fn_calculate_eic calculate_eic;
  fn_find_noise_level find_noise_level;
  fn_find_targets find_targets;
  fn_find_transitions find_transitions;
  fn_find_peaks find_peaks;
  fn_calculate_baseline calculate_baseline;
  fn_deisotope_features deisotope_features;
  fn_find_features find_features;
  fn_parse_bin parse_bin;
  fn_parse_ion_source parse_ion_source;
  fn_plan_open plan_open;
  fn_plan_eic plan_eic;
  fn_plan_find_targets plan_find_targets;
  fn_plan_find_features plan_find_features;
  fn_plan_scans plan_scans;
  fn_scans_with_arrays plan_scans_with_arrays;
  fn_scans_with_arrays get_scans_with_arrays;
  fn_plan_image plan_image;
  fn_parse_ion_path parse_ion_path;
  fn_free_ free_;
  fn_free_mzml free_mzml;
  fn_match_features match_features;
  fn_get_scans get_scans;
  fn_scan_metadata get_scan_metadata;
  fn_plan_scan_metadata plan_scan_metadata;
  fn_get_ion_image get_ion_image;
  fn_model_load model_load_path;
  fn_model_load model_load_bytes;
  fn_model_free model_free;
  fn_model_metadata model_metadata;
  fn_warp_model_load warp_model_load;
  fn_warp_model_free warp_model_free;
  fn_calibrate_rts calibrate_rts;
  fn_predict_rts_from_file predict_rts_from_file;
  fn_plan_predict_rts plan_predict_rts;
} msabi_t;

static msabi_t ABI{};
static DLIB LIB_HANDLE = NULL;

static bool ThrowIfMissing(Napi::Env env, void *fn_ptr, const char *name);

static std::atomic<size_t> OUTSTANDING_EXTERNAL{0};
static std::atomic<size_t> OUTSTANDING_HANDLES{0};

struct OwnedBuf
{
  Buf buf{nullptr, 0};

  void Reset()
  {
    if (buf.ptr && ABI.free_)
      ABI.free_(buf.ptr, buf.len);
    buf.ptr = nullptr;
    buf.len = 0;
  }

  ~OwnedBuf()
  {
    Reset();
  }

  Buf *Out()
  {
    return &buf;
  }

  Buf Release()
  {
    Buf out = buf;
    buf.ptr = nullptr;
    buf.len = 0;
    return out;
  }
};

struct ExternalBuf
{
  unsigned char *ptr;
  size_t len;
};

static void FinalizeExternalBuffer(Napi::Env, uint8_t *, ExternalBuf *ext)
{
  if (ext)
  {
    if (ext->ptr && ABI.free_)
      ABI.free_(ext->ptr, ext->len);
    delete ext;
    OUTSTANDING_EXTERNAL.fetch_sub(1, std::memory_order_relaxed);
  }
}

static void FinalizeExternalArrayBuffer(Napi::Env, void *, ExternalBuf *ext)
{
  if (ext)
  {
    if (ext->ptr && ABI.free_)
      ABI.free_(ext->ptr, ext->len);
    delete ext;
    OUTSTANDING_EXTERNAL.fetch_sub(1, std::memory_order_relaxed);
  }
}

static void FinalizeMzML(Napi::Env env, MzMLWrapper *wrapper)
{
  if (wrapper)
  {
    if (wrapper->ptr && ABI.free_mzml)
    {
      ABI.free_mzml(wrapper->ptr);
      Napi::MemoryManagement::AdjustExternalMemory(env, -(int64_t)wrapper->estimated_size);
      OUTSTANDING_HANDLES.fetch_sub(1, std::memory_order_relaxed);
    }
    delete wrapper->store;
    delete wrapper;
  }
}

static Napi::Value DisposeMzML(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.free_mzml, "free_mzml"))
    return env.Undefined();
  if (info.Length() < 1 || !info[0].IsExternal())
    return env.Undefined();

  MzMLWrapper *w = info[0].As<Napi::External<MzMLWrapper>>().Data();
  if (w && w->ptr)
  {
    if (ABI.free_mzml)
      ABI.free_mzml(w->ptr);
    Napi::MemoryManagement::AdjustExternalMemory(env, -(int64_t)w->estimated_size);
    w->ptr = nullptr;
    OUTSTANDING_HANDLES.fetch_sub(1, std::memory_order_relaxed);
  }
  return env.Undefined();
}

static MzML *GetHandle(Napi::Value val)
{
  if (!val.IsExternal())
    return nullptr;
  MzMLWrapper *w = val.As<Napi::External<MzMLWrapper>>().Data();
  if (!w || !w->ptr)
    return nullptr;
  return w->ptr;
}

static int resolve_required(void **fn, const char *name)
{
#if !defined(_WIN32)
  dlerror();
#endif
  *fn = DLSYM(LIB_HANDLE, name);
  if (!*fn)
  {
#if !defined(_WIN32)
    last_err = dlerror();
#endif
    return -1;
  }
  return 0;
}

static int abi_load(const char *path, const char **err)
{
  if (LIB_HANDLE)
  {
    DLCLOSE(LIB_HANDLE);
    LIB_HANDLE = NULL;
  }
  memset(&ABI, 0, sizeof(ABI));
#if !defined(_WIN32)
  dlerror();
#endif
  LIB_HANDLE = DLOPEN(path);
#if !defined(_WIN32)
  if (!LIB_HANDLE)
    last_err = dlerror();
#endif
  if (!LIB_HANDLE)
  {
    if (err)
      *err = last_err;
    return -1;
  }

  if (resolve_required((void **)&ABI.quantion_abi_version, "quantion_abi_version"))
    goto fail;
  if (ABI.quantion_abi_version() != QUANTION_ABI_VERSION)
  {
    last_err = "ABI version mismatch: rebuild the native addon";
    goto fail;
  }
  if (resolve_required((void **)&ABI.quantion_sizeof_peak_options, "quantion_sizeof_peak_options"))
    goto fail;
  if (ABI.quantion_sizeof_peak_options() != sizeof(CPeakOptions))
  {
    last_err = "FindPeaksOptions size mismatch: native binary and JS addon are out of sync — rebuild";
    goto fail;
  }
  if (resolve_required((void **)&ABI.parse_mzml, "parse_mzml"))
    goto fail;
  if (resolve_required((void **)&ABI.quantion_last_error, "quantion_last_error"))
    goto fail;
  if (resolve_required((void **)&ABI.parse_mzml_path, "parse_mzml_path"))
    goto fail;
  if (resolve_required((void **)&ABI.write_ion, "write_ion"))
    goto fail;
  if (resolve_required((void **)&ABI.write_mzml, "write_mzml"))
    goto fail;
  if (resolve_required((void **)&ABI.fit_peak, "fit_peak"))
    goto fail;
  if (resolve_required((void **)&ABI.draw_peaks, "draw_peaks"))
    goto fail;
  if (resolve_required((void **)&ABI.get_peak, "get_peak"))
    goto fail;
  if (resolve_required((void **)&ABI.calculate_eic, "calculate_eic"))
    goto fail;
  if (resolve_required((void **)&ABI.find_targets, "find_targets"))
    goto fail;
  if (resolve_required((void **)&ABI.find_transitions, "find_transitions"))
    goto fail;
  if (resolve_required((void **)&ABI.find_peaks, "find_peaks"))
    goto fail;

  ABI.calculate_baseline = (fn_calculate_baseline)DLSYM(LIB_HANDLE, "calculate_baseline");
  if (resolve_required((void **)&ABI.deisotope_features, "deisotope_features"))
    goto fail;

  if (resolve_required((void **)&ABI.find_features, "find_features"))
    goto fail;
  ABI.parse_ion_source = (fn_parse_ion_source)DLSYM(LIB_HANDLE, "parse_ion_source");
  ABI.plan_open = (fn_plan_open)DLSYM(LIB_HANDLE, "plan_open");
  ABI.plan_eic = (fn_plan_eic)DLSYM(LIB_HANDLE, "plan_eic");
  ABI.plan_find_targets = (fn_plan_find_targets)DLSYM(LIB_HANDLE, "plan_find_targets");
  ABI.plan_find_features = (fn_plan_find_features)DLSYM(LIB_HANDLE, "plan_find_features");
  ABI.plan_scans = (fn_plan_scans)DLSYM(LIB_HANDLE, "plan_scans");
  ABI.plan_scans_with_arrays = (fn_scans_with_arrays)DLSYM(LIB_HANDLE, "plan_scans_with_arrays");
  ABI.get_scans_with_arrays = (fn_scans_with_arrays)DLSYM(LIB_HANDLE, "get_scans_with_arrays");
  ABI.plan_image = (fn_plan_image)DLSYM(LIB_HANDLE, "plan_image");
  if (resolve_required((void **)&ABI.parse_bin, "parse_bin"))
    goto fail;
  ABI.parse_ion_path = (fn_parse_ion_path)DLSYM(LIB_HANDLE, "parse_ion_path");
  if (resolve_required((void **)&ABI.match_features, "match_features"))
    goto fail;
  if (resolve_required((void **)&ABI.get_scans, "get_scans"))
    goto fail;
  if (resolve_required((void **)&ABI.get_scan_metadata, "get_scan_metadata"))
    goto fail;
  if (resolve_required((void **)&ABI.plan_scan_metadata, "plan_scan_metadata"))
    goto fail;

  ABI.get_ion_image = (fn_get_ion_image)DLSYM(LIB_HANDLE, "get_ion_image");

  if (resolve_required((void **)&ABI.model_load_path, "model_load_path"))
    goto fail;
  if (resolve_required((void **)&ABI.model_load_bytes, "model_load_bytes"))
    goto fail;
  if (resolve_required((void **)&ABI.model_free, "model_free"))
    goto fail;
  if (resolve_required((void **)&ABI.model_metadata, "model_metadata"))
    goto fail;
  if (resolve_required((void **)&ABI.warp_model_load, "warp_model_load"))
    goto fail;
  if (resolve_required((void **)&ABI.warp_model_free, "warp_model_free"))
    goto fail;
  if (resolve_required((void **)&ABI.calibrate_rts, "calibrate_rts"))
    goto fail;
  if (resolve_required((void **)&ABI.predict_rts_from_file, "predict_rts_from_file"))
    goto fail;
  if (resolve_required((void **)&ABI.plan_predict_rts, "plan_predict_rts"))
    goto fail;

  ABI.find_noise_level = (fn_find_noise_level)DLSYM(LIB_HANDLE, "find_noise_level");
  ABI.free_ = (fn_free_)DLSYM(LIB_HANDLE, "free_");
  if (!ABI.free_)
    goto fail;
  if (resolve_required((void **)&ABI.free_mzml, "free_mzml"))
    goto fail;

  return 0;

fail:
  if (LIB_HANDLE)
  {
    DLCLOSE(LIB_HANDLE);
    LIB_HANDLE = NULL;
  }
  memset(&ABI, 0, sizeof(ABI));
  if (err)
    *err = last_err;
  return -1;
}

static bool ThrowIfMissing(Napi::Env env, void *fn_ptr, const char *name)
{
  if (fn_ptr != nullptr)
    return true;
  std::string msg = "native symbol not exported: ";
  msg += name;
  Napi::Error::New(env, msg).ThrowAsJavaScriptException();
  return false;
}

static Napi::Value ThrowRc(Napi::Env env, const char *api, int32_t rc)
{
  std::string text;
  Buf last{nullptr, 0};
  if (ABI.quantion_last_error && ABI.quantion_last_error(&last) == 0 && last.ptr)
  {
    text.assign((const char *)last.ptr, last.len);
    if (ABI.free_)
      ABI.free_(last.ptr, last.len);
  }
  if (text.empty())
  {
    text = std::to_string(rc);
    text += "\x1finternal\x1f";
    text += api;
    text += "\x1f\x1fthe call failed (this is a bug in quantion, please report it)";
  }
  Napi::Error error = Napi::Error::New(env, "quantion call failed");
  error.Value().Set("quantion", Napi::String::New(env, text));
  error.ThrowAsJavaScriptException();
  return env.Undefined();
}

static Napi::Buffer<uint8_t> TakeBuffer(Napi::Env env, Buf *buf)
{
  std::unique_ptr<ExternalBuf> held(new ExternalBuf{buf->ptr, buf->len});
  Napi::Buffer<uint8_t> out = Napi::Buffer<uint8_t>::New(env, (uint8_t *)held->ptr, held->len, FinalizeExternalBuffer, held.get());
  held.release();
  OUTSTANDING_EXTERNAL.fetch_add(1, std::memory_order_relaxed);
  buf->ptr = nullptr;
  buf->len = 0;
  return out;
}

static Napi::ArrayBuffer TakeArrayBuffer(Napi::Env env, Buf *buf)
{
  std::unique_ptr<ExternalBuf> held(new ExternalBuf{buf->ptr, buf->len});
  Napi::ArrayBuffer ab = Napi::ArrayBuffer::New(env, (void *)held->ptr, held->len, FinalizeExternalArrayBuffer, held.get());
  held.release();
  OUTSTANDING_EXTERNAL.fetch_add(1, std::memory_order_relaxed);
  buf->ptr = nullptr;
  buf->len = 0;
  return ab;
}

static double GetF64(const Napi::Object &o, const char *key, double fallback)
{
  if (!o.Has(key))
    return fallback;
  Napi::Value v = o.Get(key);
  if (v.IsNumber())
    return v.As<Napi::Number>().DoubleValue();
  return fallback;
}

static int32_t GetI32(const Napi::Object &o, const char *key, int32_t fallback)
{
  if (!o.Has(key))
    return fallback;
  Napi::Value v = o.Get(key);
  if (v.IsNumber())
    return v.As<Napi::Number>().Int32Value();
  if (v.IsBoolean())
    return v.As<Napi::Boolean>().Value() ? 1 : 0;
  return fallback;
}

static const int32_t SHAPE_GAUSSIAN = 0;
static const int32_t SHAPE_EMG = 1;

static int32_t GetShapeCode(const Napi::Object &o, const char *key)
{
  if (!o.Has(key))
    return SHAPE_EMG;
  Napi::Value v = o.Get(key);
  if (v.IsNumber())
    return v.As<Napi::Number>().Int32Value();
  if (v.IsString())
  {
    std::string name = v.As<Napi::String>().Utf8Value();
    if (name == "GAUSSIAN")
      return SHAPE_GAUSSIAN;
    return SHAPE_EMG;
  }
  return SHAPE_EMG;
}

static const CPeakOptions *ReadPeakOptionsObject(Napi::Value value, CPeakOptions *out)
{
  if (value.IsUndefined() || value.IsNull())
    return nullptr;
  if (!value.IsObject() || value.IsArray() || value.IsBuffer())
    return nullptr;
  Napi::Object o = value.As<Napi::Object>();
  out->min_integral          = GetF64(o, "minIntegral",         NAN);
  out->min_intensity         = GetF64(o, "minIntensity",        NAN);
  out->min_peak_width_points = GetI32(o, "minPeakWidthPoints",  0);
  out->shape                 = GetShapeCode(o, "shape");
  out->noise                 = GetF64(o, "noise",               NAN);
  out->auto_noise            = GetI32(o, "autoNoise",           0);
  out->auto_baseline         = GetI32(o, "autoBaseline",        0);
  out->lambda                = GetF64(o, "lambda",              0);
  out->max_iterations        = GetI32(o, "maxIterations",       0);
  out->allow_overlap         = GetI32(o, "allowOverlap",        0);
  out->min_snr               = GetF64(o, "minSnr",              NAN);
  out->min_r2                = GetF64(o, "minR2",               NAN);
  out->kernel_size           = GetI32(o, "kernelSize",         0);
  return out;
}

static const double *Float64Ptr(const Napi::Float64Array &arr)
{
  return (const double *)((const uint8_t *)arr.ArrayBuffer().Data() + arr.ByteOffset());
}

static const float *Float32Ptr(const Napi::Float32Array &arr)
{
  return (const float *)((const uint8_t *)arr.ArrayBuffer().Data() + arr.ByteOffset());
}

struct PackedIds
{
  const uint32_t *offs_ptr = nullptr;
  const uint32_t *lens_ptr = nullptr;
  const unsigned char *ids_buf_ptr = nullptr;
  size_t ids_buf_len = 0;

  std::vector<uint32_t> offs;
  std::vector<uint32_t> lens;
  std::vector<unsigned char> ids_buf;
};

static bool BuildPackedIds(Napi::Env env, const Napi::Value &value, size_t count, PackedIds *out)
{
  (void)env;
  if (value.IsUndefined() || value.IsNull())
    return true;
  if (!value.IsArray())
    return false;

  Napi::Array ids = value.As<Napi::Array>();
  if ((size_t)ids.Length() != count)
    return false;

  out->offs.assign(count, 0);
  out->lens.assign(count, 0);

  std::vector<std::string> tmp;
  tmp.resize(count);

  size_t total = 0;
  for (size_t i = 0; i < count; i++)
  {
    Napi::Value v = ids.Get((uint32_t)i);
    if (v.IsString())
    {
      tmp[i] = v.As<Napi::String>().Utf8Value();
      total += tmp[i].size();
    }
    else
    {
      tmp[i].clear();
    }
  }

  out->ids_buf.resize(total);

  size_t cur = 0;
  for (size_t i = 0; i < count; i++)
  {
    const std::string &s = tmp[i];
    out->offs[i] = (uint32_t)cur;
    out->lens[i] = (uint32_t)s.size();
    if (!s.empty())
    {
      memcpy(out->ids_buf.data() + cur, s.data(), s.size());
      cur += s.size();
    }
  }

  out->offs_ptr = out->offs.data();
  out->lens_ptr = out->lens.data();
  out->ids_buf_ptr = out->ids_buf.data();
  out->ids_buf_len = out->ids_buf.size();
  return true;
}

static Napi::Value ThrowClosed(Napi::Env env, const char *api)
{
  std::string text = "1\x1finvalid_argument\x1f";
  text += api;
  text += "\x1f" "file" "\x1f" "file is not open; it was disposed or never opened";
  Napi::Error error = Napi::Error::New(env, "quantion call failed");
  error.Value().Set("quantion", Napi::String::New(env, text));
  error.ThrowAsJavaScriptException();
  return env.Undefined();
}

static double Number(const Napi::CallbackInfo &info, size_t index)
{
  return info[index].As<Napi::Number>().DoubleValue();
}

static uint8_t Byte(const Napi::CallbackInfo &info, size_t index)
{
  return (uint8_t)info[index].As<Napi::Number>().Uint32Value();
}

static size_t Count(const Napi::CallbackInfo &info, size_t index)
{
  return (size_t)info[index].As<Napi::Number>().Int64Value();
}

static Napi::Float64Array TakeNumbers(Napi::Env env, Buf *buf)
{
  size_t count = buf->len / 8;
  Napi::ArrayBuffer bytes = TakeArrayBuffer(env, buf);
  return Napi::Float64Array::New(env, count, bytes, 0);
}

static Napi::Value ParseMzmlPath(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  std::string path = info[0].As<Napi::String>().Utf8Value();
  MzML *handle = nullptr;
  int32_t rc = ABI.parse_mzml_path(path.c_str(), &handle);
  if (rc != 0)
    return ThrowRc(env, "parse_mzml", rc);
  MzMLWrapper *wrapper = new MzMLWrapper{handle, 0, nullptr};
  Napi::External<MzMLWrapper> external = Napi::External<MzMLWrapper>::New(env, wrapper, FinalizeMzML);
  OUTSTANDING_HANDLES.fetch_add(1, std::memory_order_relaxed);
  return external;
}

static Napi::Value WriteIon(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "write_ion");
  std::string out_path;
  bool has_out = info[1].IsString();
  if (has_out)
    out_path = info[1].As<Napi::String>().Utf8Value();
  uint32_t block_size = info[5].As<Napi::Number>().Uint32Value();
  uint32_t mz_window = info[6].As<Napi::Number>().Uint32Value();
  Napi::Buffer<uint8_t> headers = info[7].As<Napi::Buffer<uint8_t>>();
  OwnedBuf out;
  int32_t rc = ABI.write_ion(handle, has_out ? out_path.c_str() : nullptr, Byte(info, 2), Byte(info, 3), Byte(info, 4),
                             block_size, mz_window, headers.Data(), headers.Length(), out.Out());
  if (rc != 0)
    return ThrowRc(env, "write_ion", rc);
  if (has_out && out.buf.ptr == nullptr)
    return env.Undefined();
  return TakeBuffer(env, out.Out());
}

static Napi::Value WriteMzml(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "write_mzml");
  std::string out_path;
  bool has_out = info[1].IsString();
  if (has_out)
    out_path = info[1].As<Napi::String>().Utf8Value();
  OwnedBuf out;
  int32_t rc = ABI.write_mzml(handle, has_out ? out_path.c_str() : nullptr, Byte(info, 2), out.Out());
  if (rc != 0)
    return ThrowRc(env, "write_mzml", rc);
  if (has_out)
    return env.Undefined();
  return TakeBuffer(env, out.Out());
}

static Napi::Value FitPeak(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  Napi::Float64Array x = info[0].As<Napi::Float64Array>();
  Napi::Float64Array y = info[1].As<Napi::Float64Array>();
  size_t count = x.ElementLength() < y.ElementLength() ? x.ElementLength() : y.ElementLength();
  OwnedBuf out;
  int32_t rc = ABI.fit_peak(Float64Ptr(x), Float64Ptr(y), count, Number(info, 2), (int32_t)Number(info, 3),
                            Number(info, 4), out.Out());
  if (rc != 0)
    return ThrowRc(env, "fit_peak", rc);
  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value DrawPeaks(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  Napi::Float64Array heights = info[0].As<Napi::Float64Array>();
  Napi::Float64Array centers = info[1].As<Napi::Float64Array>();
  Napi::Float64Array fwhms = info[2].As<Napi::Float64Array>();
  Napi::Float64Array tails = info[3].As<Napi::Float64Array>();
  OwnedBuf x;
  OwnedBuf y;
  int32_t rc = ABI.draw_peaks(Float64Ptr(heights), Float64Ptr(centers), Float64Ptr(fwhms), Float64Ptr(tails),
                              heights.ElementLength(), Number(info, 4), Number(info, 5), Count(info, 6), x.Out(), y.Out());
  if (rc != 0)
    return ThrowRc(env, "draw_peaks", rc);
  Napi::Object result = Napi::Object::New(env);
  result.Set("x", TakeNumbers(env, x.Out()));
  result.Set("y", TakeNumbers(env, y.Out()));
  return result;
}

static Napi::Value Bind(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (info.Length() < 1 || !info[0].IsString())
  {
    Napi::TypeError::New(env, "expected: path string").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  if (OUTSTANDING_EXTERNAL.load(std::memory_order_relaxed) != 0 || OUTSTANDING_HANDLES.load(std::memory_order_relaxed) != 0)
  {
    Napi::Error::New(env, "cannot rebind native library while external buffers or parsed file handles are still alive").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  std::string path = info[0].As<Napi::String>();
  const char *err = nullptr;
  if (abi_load(path.c_str(), &err) != 0)
  {
    std::string msg = "dlopen failed: ";
    if (err)
      msg += err;
    Napi::Error::New(env, msg).ThrowAsJavaScriptException();
    return env.Undefined();
  }
  return env.Undefined();
}

static Napi::Value ParseMzML(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.parse_mzml, "parse_mzml"))
    return env.Undefined();
  if (info.Length() < 1 || !info[0].IsBuffer())
    return env.Undefined();

  Napi::Buffer<uint8_t> input = info[0].As<Napi::Buffer<uint8_t>>();
  size_t input_len = input.Length();

  MzML *handle = nullptr;
  int32_t rc = ABI.parse_mzml(input.Data(), input_len, &handle);

  if (rc != 0)
    return ThrowRc(env, "parse_mzml", rc);

  MzMLWrapper *w = new MzMLWrapper{handle, input_len, nullptr};
  Napi::MemoryManagement::AdjustExternalMemory(env, (int64_t)input_len);
  Napi::External<MzMLWrapper> external = Napi::External<MzMLWrapper>::New(env, w, FinalizeMzML);
  OUTSTANDING_HANDLES.fetch_add(1, std::memory_order_relaxed);
  return external;
}


static Napi::Value GetPeak(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.get_peak, "get_peak"))
    return env.Undefined();
  if (!ThrowIfMissing(env, (void *)ABI.free_, "free_"))
    return env.Undefined();
  if (info.Length() < 4 || !info[0].IsTypedArray() || !info[1].IsTypedArray() || !info[2].IsNumber() || !info[3].IsNumber())
  {
    Napi::TypeError::New(env, "expected: (Float64Array x, Float64Array y, number targetRt, number rtRange, Buffer|null options?)").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  Napi::Float64Array x_arr = info[0].As<Napi::Float64Array>();
  Napi::Float64Array y_arr = info[1].As<Napi::Float64Array>();
  double target_rt = info[2].As<Napi::Number>().DoubleValue();
  double rt_range = info[3].As<Napi::Number>().DoubleValue();

  size_t n = x_arr.ElementLength();
  if (y_arr.ElementLength() != n)
  {
    Napi::TypeError::New(env, "x/y length mismatch").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  CPeakOptions opts;
  const CPeakOptions *p_opts = nullptr;
  if (info.Length() > 4)
    p_opts = ReadPeakOptionsObject(info[4], &opts);

  const double *x_ptr = Float64Ptr(x_arr);
  const double *y_ptr = Float64Ptr(y_arr);

  OwnedBuf out;
  int32_t rc = ABI.get_peak(x_ptr, y_ptr, n, target_rt, rt_range, p_opts, out.Out());
  if (rc != 0)
    return ThrowRc(env, "get_peak", rc);

  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value CalculateEic(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "calculate_eic");
  OwnedBuf x;
  OwnedBuf y;
  int32_t rc = ABI.calculate_eic(handle, Number(info, 1), Number(info, 2), Number(info, 3), Number(info, 4),
                                 Number(info, 5), Byte(info, 6), x.Out(), y.Out());
  if (rc != 0)
    return ThrowRc(env, "calculate_eic", rc);
  Napi::Object result = Napi::Object::New(env);
  result.Set("x", TakeNumbers(env, x.Out()));
  result.Set("y", TakeNumbers(env, y.Out()));
  return result;
}

static Napi::Value FindNoiseLevel(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.find_noise_level, "find_noise_level"))
    return env.Undefined();
  if (info.Length() < 1 || !info[0].IsTypedArray())
  {
    Napi::TypeError::New(env, "expected: (Float32Array y)").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  Napi::Float32Array y_arr = info[0].As<Napi::Float32Array>();
  const float *y_ptr = Float32Ptr(y_arr);
  size_t width = 0;
  double intensity = 0.0;
  int32_t rc = ABI.find_noise_level(y_ptr, y_arr.ElementLength(), &width, &intensity);
  if (rc != 0)
    return ThrowRc(env, "find_noise_level", rc);
  Napi::Object out = Napi::Object::New(env);
  out.Set("width", Napi::Number::New(env, (double)width));
  out.Set("intensity", Napi::Number::New(env, intensity));
  return out;
}

static Napi::Value FindTargets(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "find_targets");
  Napi::Float64Array rts = info[1].As<Napi::Float64Array>();
  Napi::Float64Array mzs = info[2].As<Napi::Float64Array>();
  Napi::Float64Array ranges = info[3].As<Napi::Float64Array>();
  size_t count = rts.ElementLength();
  PackedIds packed;
  BuildPackedIds(env, info[4], count, &packed);
  CPeakOptions options;
  const CPeakOptions *options_pointer = ReadPeakOptionsObject(info[10], &options);
  OwnedBuf out;
  int32_t rc = ABI.find_targets(handle, Float64Ptr(rts), Float64Ptr(mzs), Float64Ptr(ranges), packed.offs_ptr,
                                packed.lens_ptr, packed.ids_buf_ptr, packed.ids_buf_len, count, Number(info, 5),
                                Number(info, 6), Number(info, 7), Number(info, 8), Byte(info, 9), options_pointer,
                                Count(info, 11), out.Out());
  if (rc != 0)
    return ThrowRc(env, "find_targets", rc);
  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value FindTransitions(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.find_transitions, "find_transitions"))
    return env.Undefined();
  if (info.Length() < 7)
    return env.Undefined();

  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "find_transitions");

  if (!info[1].IsTypedArray() || !info[2].IsTypedArray() || !info[3].IsTypedArray() || !info[4].IsTypedArray())
  {
    Napi::TypeError::New(env, "expected: (External, Float64Array precursors, Float64Array products, Float64Array rts, Float64Array ranges, ...)").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  Napi::Float64Array precursors = info[1].As<Napi::Float64Array>();
  Napi::Float64Array products = info[2].As<Napi::Float64Array>();
  Napi::Float64Array rts = info[3].As<Napi::Float64Array>();
  Napi::Float64Array rng = info[4].As<Napi::Float64Array>();
  size_t count = rts.ElementLength();
  PackedIds packed;
  if (!BuildPackedIds(env, info[5], count, &packed))
  {
    Napi::TypeError::New(env, "ids length must equal target count").ThrowAsJavaScriptException();
    return env.Undefined();
  }
  CPeakOptions opts;
  const CPeakOptions *p_opts = ReadPeakOptionsObject(info[6], &opts);
  size_t threads = info.Length() > 7 ? info[7].As<Napi::Number>().Uint32Value() : 1;
  OwnedBuf out;
  int32_t rc = ABI.find_transitions(handle, Float64Ptr(precursors), Float64Ptr(products), Float64Ptr(rts), Float64Ptr(rng),
                                        packed.offs_ptr, packed.lens_ptr, packed.ids_buf_ptr, packed.ids_buf_len,
                                        count, p_opts, threads, out.Out());
  if (rc != 0)
    return ThrowRc(env, "find_transitions", rc);
  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value FindPeaks(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.find_peaks, "find_peaks"))
    return env.Undefined();
  if (!ThrowIfMissing(env, (void *)ABI.free_, "free_"))
    return env.Undefined();
  if (info.Length() < 2 || !info[0].IsTypedArray() || !info[1].IsTypedArray())
  {
    Napi::TypeError::New(env, "expected: (Float64Array x, Float64Array y, Buffer|null options?)").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  Napi::Float64Array x_arr = info[0].As<Napi::Float64Array>();
  Napi::Float64Array y_arr = info[1].As<Napi::Float64Array>();
  size_t n = x_arr.ElementLength();
  if (y_arr.ElementLength() != n)
  {
    Napi::TypeError::New(env, "x/y length mismatch").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  CPeakOptions opts;
  const CPeakOptions *p_opts = nullptr;
  if (info.Length() > 2)
    p_opts = ReadPeakOptionsObject(info[2], &opts);

  const double *x_ptr = Float64Ptr(x_arr);
  const double *y_ptr = Float64Ptr(y_arr);

  OwnedBuf out;
  int32_t rc = ABI.find_peaks(x_ptr, y_ptr, n, p_opts, out.Out());
  if (rc != 0)
    return ThrowRc(env, "find_peaks", rc);

  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value CalculateBaseline(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  Napi::Float64Array y = info[0].As<Napi::Float64Array>();
  OwnedBuf out;
  int32_t rc = ABI.calculate_baseline(Float64Ptr(y), y.ElementLength(), Number(info, 1), Count(info, 2), out.Out());
  if (rc != 0)
    return ThrowRc(env, "calculate_baseline", rc);
  return TakeNumbers(env, out.Out());
}

static Napi::Value DeisotopeFeatures(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  Napi::Float64Array values = info[0].As<Napi::Float64Array>();
  OwnedBuf out;
  int32_t rc = ABI.deisotope_features(Float64Ptr(values), Count(info, 1), Number(info, 2), Number(info, 3), out.Out());
  if (rc != 0)
    return ThrowRc(env, "deisotope_features", rc);
  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value FindFeatures(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "find_features");
  CPeakOptions options;
  const CPeakOptions *options_pointer = ReadPeakOptionsObject(info[9], &options);
  OwnedBuf out;
  int32_t rc = ABI.find_features(handle, Number(info, 1), Number(info, 2), Number(info, 3), Number(info, 4),
                                 Byte(info, 5), Number(info, 6), Number(info, 7), Number(info, 8), options_pointer,
                                 Count(info, 10), out.Out());
  if (rc != 0)
    return ThrowRc(env, "find_features", rc);
  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value ParseBin(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.parse_bin, "parse_bin"))
    return env.Undefined();
  if (info.Length() < 1 || !info[0].IsBuffer())
    return env.Undefined();

  Napi::Buffer<uint8_t> input = info[0].As<Napi::Buffer<uint8_t>>();
  size_t input_len = input.Length();
  size_t max_cache = info.Length() > 1 && info[1].IsNumber()
                         ? (size_t)info[1].As<Napi::Number>().Int64Value()
                         : 0;

  MzML *handle = nullptr;
  int32_t rc = ABI.parse_bin(input.Data(), input_len, max_cache, &handle);

  if (rc != 0)
    return ThrowRc(env, "parse_bin", rc);

  MzMLWrapper *w = new MzMLWrapper{handle, input_len, nullptr};
  Napi::MemoryManagement::AdjustExternalMemory(env, (int64_t)input_len);
  Napi::External<MzMLWrapper> external = Napi::External<MzMLWrapper>::New(env, w, FinalizeMzML);
  OUTSTANDING_HANDLES.fetch_add(1, std::memory_order_relaxed);
  return external;
}

static Napi::Array RangesToArray(Napi::Env env, const Buf &out)
{
  size_t count = out.len / 16;
  Napi::Array ranges = Napi::Array::New(env, count);
  for (size_t index = 0; index < count; index++)
  {
    uint64_t offset = 0;
    uint64_t length = 0;
    std::memcpy(&offset, out.ptr + index * 16, 8);
    std::memcpy(&length, out.ptr + index * 16 + 8, 8);
    Napi::Object range = Napi::Object::New(env);
    range.Set("offset", Napi::BigInt::New(env, offset));
    range.Set("length", Napi::BigInt::New(env, length));
    ranges.Set(index, range);
  }
  return ranges;
}

static Napi::Value PlanOpen(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.plan_open, "plan_open"))
    return env.Undefined();
  if (info.Length() < 1 || !info[0].IsBuffer())
    return env.Undefined();

  Napi::Buffer<uint8_t> header = info[0].As<Napi::Buffer<uint8_t>>();
  OwnedBuf out;
  int32_t rc = ABI.plan_open(header.Data(), header.Length(), out.Out());
  if (rc != 0)
    return ThrowRc(env, "plan_open", rc);

  return RangesToArray(env, out.buf);
}

static Napi::Value PlanEic(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "calculate_eic");
  OwnedBuf out;
  int32_t rc = ABI.plan_eic(handle, Number(info, 1), Number(info, 2), Number(info, 3), Number(info, 4), Number(info, 5),
                            Byte(info, 6), out.Out());
  if (rc != 0)
    return ThrowRc(env, "calculate_eic", rc);
  return RangesToArray(env, out.buf);
}

static Napi::Value PlanFindTargets(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "find_targets");
  Napi::Float64Array mzs = info[1].As<Napi::Float64Array>();
  OwnedBuf out;
  int32_t rc = ABI.plan_find_targets(handle, Float64Ptr(mzs), mzs.ElementLength(), Number(info, 2), Number(info, 3),
                                     Number(info, 4), Number(info, 5), Byte(info, 6), out.Out());
  if (rc != 0)
    return ThrowRc(env, "find_targets", rc);
  return RangesToArray(env, out.buf);
}

static Napi::Value PlanFindFeatures(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "find_features");
  OwnedBuf out;
  int32_t rc = ABI.plan_find_features(handle, Number(info, 1), Number(info, 2), Number(info, 3), Number(info, 4),
                                      Byte(info, 5), Number(info, 6), Number(info, 7), Number(info, 8), out.Out());
  if (rc != 0)
    return ThrowRc(env, "find_features", rc);
  return RangesToArray(env, out.buf);
}

static Napi::Value PlanScans(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "get_scans");
  OwnedBuf out;
  int32_t rc = ABI.plan_scans(handle, Byte(info, 1), Number(info, 2), Number(info, 3), Byte(info, 4), Byte(info, 5),
                              out.Out());
  if (rc != 0)
    return ThrowRc(env, "get_scans", rc);
  return RangesToArray(env, out.buf);
}

static Napi::Value PlanImage(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.plan_image, "plan_image"))
    return env.Undefined();
  if (info.Length() < 4 || !info[0].IsExternal())
    return env.Undefined();

  MzMLWrapper *wrapper = info[0].As<Napi::External<MzMLWrapper>>().Data();
  OwnedBuf out;
  int32_t rc = ABI.plan_image(
      wrapper->ptr,
      info[1].As<Napi::Number>().DoubleValue(),
      info[2].As<Napi::Number>().DoubleValue(),
      (uint8_t)info[3].As<Napi::Number>().Uint32Value(),
      out.Out());
  if (rc != 0)
    return ThrowRc(env, "plan_image", rc);

  return RangesToArray(env, out.buf);
}

static Napi::Value ParseIonSource(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.parse_ion_source, "parse_ion_source"))
    return env.Undefined();
  if (info.Length() < 1 || !info[0].IsFunction())
    return env.Undefined();

  size_t max_cache = info.Length() > 1 && info[1].IsNumber()
                         ? (size_t)info[1].As<Napi::Number>().Int64Value()
                         : 0;

  RangeStore *store = new RangeStore{env, Napi::Persistent(info[0].As<Napi::Function>()), std::this_thread::get_id()};

  MzML *handle = nullptr;
  int32_t rc = ABI.parse_ion_source(serve_range, store, max_cache, &handle);
  if (rc != 0)
  {
    delete store;
    return ThrowRc(env, "parse_ion_source", rc);
  }

  MzMLWrapper *w = new MzMLWrapper{handle, 0, store};
  Napi::External<MzMLWrapper> external = Napi::External<MzMLWrapper>::New(env, w, FinalizeMzML);
  OUTSTANDING_HANDLES.fetch_add(1, std::memory_order_relaxed);
  return external;
}

static Napi::Value ParseIonPath(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.parse_ion_path, "parse_ion_path"))
    return env.Undefined();

  if (info.Length() < 1 || !info[0].IsString())
  {
    Napi::TypeError::New(env, "expected: (string path, number maxCacheSize?)").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  std::string file_path = info[0].As<Napi::String>().Utf8Value();
  size_t cache_size = info.Length() > 1 && info[1].IsNumber()
                          ? (size_t)info[1].As<Napi::Number>().Int64Value()
                          : 0;

  MzML *file_handle = nullptr;
  int32_t code = ABI.parse_ion_path(file_path.c_str(), cache_size, &file_handle);
  if (code != 0)
    return ThrowRc(env, "parse_ion_path", code);

  MzMLWrapper *file_wrapper = new MzMLWrapper{file_handle, 0, nullptr};
  Napi::External<MzMLWrapper> external = Napi::External<MzMLWrapper>::New(env, file_wrapper, FinalizeMzML);
  OUTSTANDING_HANDLES.fetch_add(1, std::memory_order_relaxed);
  return external;
}

static Napi::Value MatchFeatures(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  std::string directory = info[0].As<Napi::String>().Utf8Value();
  CPeakOptions options;
  const CPeakOptions *options_pointer = ReadPeakOptionsObject(info[13], &options);
  OwnedBuf out;
  int32_t rc = ABI.match_features(directory.c_str(), Number(info, 1), Number(info, 2), Number(info, 3), Number(info, 4),
                                  Byte(info, 5), Number(info, 6), Number(info, 7), Number(info, 8), Number(info, 9),
                                  Number(info, 10), Number(info, 11), Number(info, 12), options_pointer,
                                  Count(info, 14), out.Out());
  if (rc != 0)
    return ThrowRc(env, "match_features", rc);
  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value GetScans(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "get_scans");
  OwnedBuf out;
  int32_t rc = ABI.get_scans(handle, Byte(info, 1), Number(info, 2), Number(info, 3), Byte(info, 4), Byte(info, 5),
                             out.Out());
  if (rc != 0)
    return ThrowRc(env, "get_scans", rc);
  return TakeArrayBuffer(env, out.Out());
}

static int32_t CallScansWithArrays(const Napi::CallbackInfo &info, fn_scans_with_arrays call, MzML *handle, Buf *out)
{
  Napi::Uint32Array accessions = info[6].As<Napi::Uint32Array>();
  return call(handle, Byte(info, 1), Number(info, 2), Number(info, 3), Byte(info, 4), Byte(info, 5),
              accessions.Data(), accessions.ElementLength(), out);
}

static Napi::Value PlanScansWithArrays(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.plan_scans_with_arrays, "plan_scans_with_arrays"))
    return env.Null();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "get_scans");
  OwnedBuf out;
  int32_t rc = CallScansWithArrays(info, ABI.plan_scans_with_arrays, handle, out.Out());
  if (rc != 0)
    return ThrowRc(env, "get_scans", rc);
  return RangesToArray(env, out.buf);
}

static Napi::Value GetScansWithArrays(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.get_scans_with_arrays, "get_scans_with_arrays"))
    return env.Null();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "get_scans");
  OwnedBuf out;
  int32_t rc = CallScansWithArrays(info, ABI.get_scans_with_arrays, handle, out.Out());
  if (rc != 0)
    return ThrowRc(env, "get_scans", rc);
  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value PlanScanMetadata(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "get_scan_metadata");
  OwnedBuf out;
  int32_t rc = ABI.plan_scan_metadata(handle, Count(info, 1), out.Out());
  if (rc != 0)
    return ThrowRc(env, "get_scan_metadata", rc);
  return RangesToArray(env, out.buf);
}

static Napi::Value GetScanMetadata(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "get_scan_metadata");
  std::string tags = info[2].As<Napi::String>().Utf8Value();
  OwnedBuf out;
  int32_t rc = ABI.get_scan_metadata(handle, Count(info, 1), tags.c_str(), out.Out());
  if (rc != 0)
    return ThrowRc(env, "get_scan_metadata", rc);
  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value GetIonImage(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (!ThrowIfMissing(env, (void *)ABI.get_ion_image, "get_ion_image"))
    return env.Undefined();

  MzML *handle = GetHandle(info[0]);
  if (!handle)
    return ThrowClosed(env, "get_ion_image");

  if (info.Length() < 4 || !info[1].IsNumber() || !info[2].IsNumber() || !info[3].IsNumber())
  {
    Napi::TypeError::New(env, "expected: (External handle, number mz, number tolerance, number level)")
        .ThrowAsJavaScriptException();
    return env.Undefined();
  }

  double mz = info[1].As<Napi::Number>().DoubleValue();
  double tolerance = info[2].As<Napi::Number>().DoubleValue();
  uint8_t level = (uint8_t)info[3].As<Napi::Number>().Uint32Value();

  OwnedBuf out;
  int32_t rc = ABI.get_ion_image(handle, mz, tolerance, level, out.Out());
  if (rc != 0)
    return ThrowRc(env, "get_ion_image", rc);

  return TakeArrayBuffer(env, out.Out());
}

struct ModelWrapper
{
  Model *ptr;
};

static void FinalizeModel(Napi::Env, ModelWrapper *wrapper)
{
  if (wrapper)
  {
    if (wrapper->ptr)
      ABI.model_free(wrapper->ptr);
    delete wrapper;
  }
}

static Model *GetModel(Napi::Value value)
{
  if (!value.IsExternal())
    return nullptr;
  ModelWrapper *wrapper = value.As<Napi::External<ModelWrapper>>().Data();
  return wrapper ? wrapper->ptr : nullptr;
}

struct WarpModelWrapper
{
  WarpModel *ptr;
};

static void FinalizeWarpModel(Napi::Env, WarpModelWrapper *wrapper)
{
  if (wrapper)
  {
    if (wrapper->ptr)
      ABI.warp_model_free(wrapper->ptr);
    delete wrapper;
  }
}

static WarpModel *GetWarpModel(Napi::Value value)
{
  if (!value.IsExternal())
    return nullptr;
  WarpModelWrapper *wrapper = value.As<Napi::External<WarpModelWrapper>>().Data();
  return wrapper ? wrapper->ptr : nullptr;
}

static Napi::Value ThrowWarpError(Napi::Env env, const char *api, int32_t rc, const Buf &)
{
  return ThrowRc(env, api, rc);
}

static Napi::Value WrapModel(Napi::Env env, int32_t rc, Model *model, const Buf &message)
{
  if (rc != 0)
    return ThrowWarpError(env, "load_model", rc, message);
  return Napi::External<ModelWrapper>::New(env, new ModelWrapper{model}, FinalizeModel);
}

static Napi::Value LoadModelPath(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (info.Length() < 1 || !info[0].IsString())
  {
    Napi::TypeError::New(env, "expected: (string path)").ThrowAsJavaScriptException();
    return env.Undefined();
  }
  std::string path = info[0].As<Napi::String>().Utf8Value();
  Model *model = nullptr;
  OwnedBuf message;
  int32_t rc = ABI.model_load_path((const uint8_t *)path.data(), path.size(), &model, message.Out());
  return WrapModel(env, rc, model, message.buf);
}

static Napi::Value LoadModelBytes(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  if (info.Length() < 1 || !info[0].IsBuffer())
  {
    Napi::TypeError::New(env, "expected: (Buffer bytes)").ThrowAsJavaScriptException();
    return env.Undefined();
  }
  Napi::Buffer<uint8_t> bytes = info[0].As<Napi::Buffer<uint8_t>>();
  Model *model = nullptr;
  OwnedBuf message;
  int32_t rc = ABI.model_load_bytes(bytes.Data(), bytes.Length(), &model, message.Out());
  return WrapModel(env, rc, model, message.buf);
}

static Napi::Value DisposeModel(const Napi::CallbackInfo &info)
{
  if (info.Length() < 1 || !info[0].IsExternal())
    return info.Env().Undefined();
  ModelWrapper *wrapper = info[0].As<Napi::External<ModelWrapper>>().Data();
  if (wrapper && wrapper->ptr)
  {
    ABI.model_free(wrapper->ptr);
    wrapper->ptr = nullptr;
  }
  return info.Env().Undefined();
}

static Napi::Value ModelMetadata(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  Model *model = info.Length() > 0 ? GetModel(info[0]) : nullptr;
  if (!model)
  {
    Napi::TypeError::New(env, "expected: (External model)").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  OwnedBuf out;
  int32_t rc = ABI.model_metadata(model, out.Out());
  if (rc != 0)
    return ThrowRc(env, "model_metadata", rc);
  return TakeArrayBuffer(env, out.Out());
}

static Napi::Value LoadWarp(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  Model *model = info.Length() > 0 ? GetModel(info[0]) : nullptr;
  if (!model)
  {
    Napi::TypeError::New(env, "expected: (External model)").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  WarpModel *warp = nullptr;
  OwnedBuf message;
  int32_t rc = ABI.warp_model_load(model, &warp, message.Out());
  if (rc != 0)
    return ThrowWarpError(env, "load_warp", rc, message.buf);
  return Napi::External<WarpModelWrapper>::New(env, new WarpModelWrapper{warp}, FinalizeWarpModel);
}

static Napi::Value DisposeWarpModel(const Napi::CallbackInfo &info)
{
  if (info.Length() < 1 || !info[0].IsExternal())
    return info.Env().Undefined();
  WarpModelWrapper *wrapper = info[0].As<Napi::External<WarpModelWrapper>>().Data();
  if (wrapper && wrapper->ptr)
  {
    ABI.warp_model_free(wrapper->ptr);
    wrapper->ptr = nullptr;
  }
  return info.Env().Undefined();
}

static Napi::Value CalibrateRts(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  WarpModel *warp = info.Length() > 0 ? GetWarpModel(info[0]) : nullptr;
  if (!warp || info.Length() < 6 || !info[1].IsArray() || !info[2].IsArray() || !info[4].IsTypedArray() ||
      !info[5].IsTypedArray())
  {
    Napi::TypeError::New(env, "expected: (External warp, External[] files, number[] listLengths, string[] ids, "
                              "Float64Array mzs, Float64Array rts)")
        .ThrowAsJavaScriptException();
    return env.Undefined();
  }
  Napi::Array fileValues = info[1].As<Napi::Array>();
  Napi::Array lengthValues = info[2].As<Napi::Array>();
  std::vector<const MzML *> files(fileValues.Length());
  for (uint32_t i = 0; i < fileValues.Length(); i++)
  {
    files[i] = GetHandle(fileValues.Get(i));
    if (!files[i])
    {
      Napi::TypeError::New(env, "files must be loaded sample files").ThrowAsJavaScriptException();
      return env.Undefined();
    }
  }
  std::vector<size_t> lengths(lengthValues.Length());
  for (uint32_t i = 0; i < lengthValues.Length(); i++)
    lengths[i] = (size_t)lengthValues.Get(i).As<Napi::Number>().Int64Value();

  Napi::Float64Array mzs = info[4].As<Napi::Float64Array>();
  Napi::Float64Array rts = info[5].As<Napi::Float64Array>();
  size_t count = mzs.ElementLength();
  PackedIds packed;
  if (rts.ElementLength() != count || !BuildPackedIds(env, info[3], count, &packed))
  {
    Napi::TypeError::New(env, "ids, mzs and rts must have the same length").ThrowAsJavaScriptException();
    return env.Undefined();
  }

  OwnedBuf out;
  int32_t rc = ABI.calibrate_rts(warp, files.data(), files.size(), lengths.data(), lengths.size(), packed.offs_ptr, packed.lens_ptr,
                                 packed.ids_buf_ptr, packed.ids_buf_len, Float64Ptr(mzs), Float64Ptr(rts), count,
                                 out.Out());
  if (rc != 0)
    return ThrowWarpError(env, "calibrate", rc, out.buf);
  return TakeArrayBuffer(env, out.Out());
}

static bool ReadPredictArguments(const Napi::CallbackInfo &info, WarpModel **warp, MzML **file,
                                 Napi::Float64Array *mzs, Napi::Float64Array *rts)
{
  Napi::Env env = info.Env();
  *warp = info.Length() > 0 ? GetWarpModel(info[0]) : nullptr;
  *file = info.Length() > 1 ? GetHandle(info[1]) : nullptr;
  if (!*warp || !*file || info.Length() < 4 || !info[2].IsTypedArray() || !info[3].IsTypedArray())
  {
    Napi::TypeError::New(env, "expected: (External warp, External file, Float64Array mzs, Float64Array rts)")
        .ThrowAsJavaScriptException();
    return false;
  }
  *mzs = info[2].As<Napi::Float64Array>();
  *rts = info[3].As<Napi::Float64Array>();
  if (mzs->ElementLength() != rts->ElementLength())
  {
    Napi::TypeError::New(env, "mzs and rts must have the same length").ThrowAsJavaScriptException();
    return false;
  }
  return true;
}

static Napi::Value PredictRts(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  WarpModel *warp = nullptr;
  MzML *file = nullptr;
  Napi::Float64Array mzs, rts;
  if (!ReadPredictArguments(info, &warp, &file, &mzs, &rts))
    return env.Undefined();

  OwnedBuf out;
  int32_t rc = ABI.predict_rts_from_file(warp, file, Float64Ptr(mzs), Float64Ptr(rts), mzs.ElementLength(), out.Out());
  if (rc != 0)
    return ThrowWarpError(env, "predict_rts", rc, out.buf);
  size_t count = out.buf.len / 8;
  return Napi::Float64Array::New(env, count, TakeArrayBuffer(env, out.Out()), 0);
}

static Napi::Value PlanPredictRts(const Napi::CallbackInfo &info)
{
  Napi::Env env = info.Env();
  WarpModel *warp = nullptr;
  MzML *file = nullptr;
  Napi::Float64Array mzs, rts;
  if (!ReadPredictArguments(info, &warp, &file, &mzs, &rts))
    return env.Undefined();

  OwnedBuf out;
  int32_t rc = ABI.plan_predict_rts(warp, file, Float64Ptr(mzs), Float64Ptr(rts), mzs.ElementLength(), out.Out());
  if (rc != 0)
    return ThrowWarpError(env, "plan_predict_rts", rc, out.buf);
  return RangesToArray(env, out.buf);
}

static Napi::Object Init(Napi::Env env, Napi::Object exports)
{
  exports.Set("bind", Napi::Function::New(env, Bind));
  exports.Set("parseMzml", Napi::Function::New(env, ParseMzML));
  exports.Set("parseMzmlPath", Napi::Function::New(env, ParseMzmlPath));
  exports.Set("writeIon", Napi::Function::New(env, WriteIon));
  exports.Set("writeMzml", Napi::Function::New(env, WriteMzml));
  exports.Set("fitPeak", Napi::Function::New(env, FitPeak));
  exports.Set("drawPeaks", Napi::Function::New(env, DrawPeaks));
  exports.Set("planOpen", Napi::Function::New(env, PlanOpen));
  exports.Set("planEic", Napi::Function::New(env, PlanEic));
  exports.Set("planScans", Napi::Function::New(env, PlanScans));
  exports.Set("planScansWithArrays", Napi::Function::New(env, PlanScansWithArrays));
  exports.Set("getScansWithArrays", Napi::Function::New(env, GetScansWithArrays));
  exports.Set("planFindTargets", Napi::Function::New(env, PlanFindTargets));
  exports.Set("planFindFeatures", Napi::Function::New(env, PlanFindFeatures));
  exports.Set("planImage", Napi::Function::New(env, PlanImage));
  exports.Set("parseIonSource", Napi::Function::New(env, ParseIonSource));
  exports.Set("getPeak", Napi::Function::New(env, GetPeak));
  exports.Set("calculateEic", Napi::Function::New(env, CalculateEic));
  exports.Set("findNoiseLevel", Napi::Function::New(env, FindNoiseLevel));
  exports.Set("findTargets", Napi::Function::New(env, FindTargets));
  exports.Set("findTransitions", Napi::Function::New(env, FindTransitions));
  exports.Set("findPeaks", Napi::Function::New(env, FindPeaks));
  exports.Set("calculateBaseline", Napi::Function::New(env, CalculateBaseline));
  exports.Set("deisotopeFeatures", Napi::Function::New(env, DeisotopeFeatures));
  exports.Set("findFeatures", Napi::Function::New(env, FindFeatures));
  exports.Set("parseBin", Napi::Function::New(env, ParseBin));
  exports.Set("parseIonPath", Napi::Function::New(env, ParseIonPath));
  exports.Set("dispose", Napi::Function::New(env, DisposeMzML));
  exports.Set("matchFeatures", Napi::Function::New(env, MatchFeatures));
  exports.Set("getScans", Napi::Function::New(env, GetScans));
  exports.Set("planScanMetadata", Napi::Function::New(env, PlanScanMetadata));
  exports.Set("getScanMetadata", Napi::Function::New(env, GetScanMetadata));
  exports.Set("getIonImage", Napi::Function::New(env, GetIonImage));
  exports.Set("loadModelPath", Napi::Function::New(env, LoadModelPath));
  exports.Set("loadModelBytes", Napi::Function::New(env, LoadModelBytes));
  exports.Set("disposeModel", Napi::Function::New(env, DisposeModel));
  exports.Set("modelMetadata", Napi::Function::New(env, ModelMetadata));
  exports.Set("loadWarp", Napi::Function::New(env, LoadWarp));
  exports.Set("disposeWarpModel", Napi::Function::New(env, DisposeWarpModel));
  exports.Set("calibrateRts", Napi::Function::New(env, CalibrateRts));
  exports.Set("predictRts", Napi::Function::New(env, PredictRts));
  exports.Set("planPredictRts", Napi::Function::New(env, PlanPredictRts));
  return exports;
}

NODE_API_MODULE(quantion, Init)
