
#include "HippoGui.h"

///

enum
{
	MAX_CONTROLS = 1024,
};

HippoGuiState g_hippoGuiState;
static int s_controlId;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

typedef enum HippoDrawType
{
	DRAWTYPE_NONE,
	DRAWTYPE_FILL,
	DRAWTYPE_IMAGE,
} HippoDrawType;

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

struct HippoControlInfo
{
	HippoDrawType type;

	int x;
	int y;
	int width;
	int height;

	uint32_t color;
	void* imageData;
};

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

//HippoControlInfo g_controls[MAX_CONTROLS];

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void HippoGui_begin()
{
	s_controlId = 1;
	//memset(&g_controls[0], 0, sizeof(HippoControlInfo));
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

void HippoGui_end()
{
}

