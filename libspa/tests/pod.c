#include <stdarg.h>

#include <spa/pod/builder.h>
#include <spa/debug/pod.h>
#include <spa/param/audio/format-utils.h>

int build_none(uint8_t *buffer, size_t len)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_none(&b);
}

int build_bool(uint8_t *buffer, size_t len, bool boolean)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_bool(&b, boolean);
}

int build_id(uint8_t *buffer, size_t len, uint32_t id)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_id(&b, id);
}

int build_int(uint8_t *buffer, size_t len, int32_t integer)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_int(&b, integer);
}

int build_long(uint8_t *buffer, size_t len, int64_t integer)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_long(&b, integer);
}

int build_float(uint8_t *buffer, size_t len, float f)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_float(&b, f);
}

int build_double(uint8_t *buffer, size_t len, double d)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_double(&b, d);
}

int build_string(uint8_t *buffer, size_t len, const char *string)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_string(&b, string);
}

int build_bytes(uint8_t *buffer, size_t len, const void *bytes, size_t bytes_len)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_bytes(&b, bytes, bytes_len);
}

int build_rectangle(uint8_t *buffer, size_t len, uint32_t width, uint32_t height)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_rectangle(&b, width, height);
}

int build_fraction(uint8_t *buffer, size_t len, uint32_t num, uint32_t denom)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_fraction(&b, num, denom);
}

int build_array(uint8_t *buffer, size_t len, uint32_t child_size, uint32_t child_type, uint32_t n_elems, const void *elems)
{
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);
	return spa_pod_builder_array(&b, child_size, child_type, n_elems, elems);
}

void build_test_struct(
	uint8_t *buffer, size_t len, int32_t num, const char *string, uint32_t rect_width, uint32_t rect_height)
{
	struct spa_pod_frame outer, inner;
	struct spa_pod_builder b = SPA_POD_BUILDER_INIT(buffer, len);

	spa_pod_builder_push_struct(&b, &outer);
	spa_pod_builder_int(&b, num);
	spa_pod_builder_string(&b, string);

	spa_pod_builder_push_struct(&b, &inner);
	spa_pod_builder_rectangle(&b, rect_width, rect_height);

	spa_pod_builder_pop(&b, &inner);
	spa_pod_builder_pop(&b, &outer);
}

void print_pod(const struct spa_pod *pod)
{
	spa_debug_pod(0, NULL, pod);
}
